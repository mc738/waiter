use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::fs;
use std::fs::read_to_string;
use std::process::{Command, Output};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        //println!("Connection established!");
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    
    stream.read(&mut buffer).unwrap();
    
    println!("Request: {}", String::from_utf8_lossy(&buffer[..]));
    
    let home = b"GET / HTTP/1.1\r\n";
    
    if buffer.starts_with(home) {
        let contents = fs::read_to_string("index.html").unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n{}",
            contents.len(),
            contents
        );
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
    else {
        let output = get_details();
        
        let contents = String::from_utf8_lossy(&*output.stdout);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}",
            contents.len(),
            contents
            
        );
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

fn get_details() -> Output {
    Command::new("sh")
        .arg("-c")
        .arg("lscpu")
        .output()
        .expect("failed to execute process")
}
