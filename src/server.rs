use std::io::prelude::*;
use std::{fs, thread};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Output};
use std::sync::mpsc::channel;
use regex::Regex;
use serde::de::Unexpected::Str;
use uuid;
use uuid::Uuid;
use crate::connection_pool::ConnectionPool;
use crate::http::{HttpRequest, HttpRequestHeader, HttpResponse};
use crate::http::HttpVerb::GET;
use crate::logging::logging::{Log, Logger};
use crate::orchestration::Orchestrator;
use crate::routing::RouteMap;


pub struct Server;

pub struct ConnectionContext {
    id: Uuid,
    slug: String,
    from: String,
}

impl Server {
    pub fn start(routes: RouteMap) {
        let log = Log::create().unwrap();
        let logger = log.get_logger();
        let (job_sender, job_receiver) = channel();

        let orch_logger = log.get_logger();
        let _ = thread::spawn(|| {
            Orchestrator::run(job_receiver, orch_logger)
        });

        let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

        let connection_pool = ConnectionPool::new(4, logger, job_sender.clone());

        //let logger = log.get_logger();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let remote = stream.peer_addr().unwrap();
            let context = ConnectionContext::new(String::from(remote.ip().to_string()));
            let logger = log.get_logger();

            logger.log_info(format!("{}", context.slug), format!("Request received from {}", context.from));
            
            let rm = routes.clone();
            connection_pool.execute(|| {
                handle_connection(stream, logger, context, rm)
            });
        }
    }
}

impl ConnectionContext {
    pub fn new(from: String) -> ConnectionContext {
        let id = Uuid::new_v4();
        let slug = String::from(Uuid::new_v4().to_string().split_at(6).0);
        ConnectionContext { id, slug, from }
    }
}

fn handle_connection(mut stream: TcpStream, logger: Logger, context: ConnectionContext, route_map: RouteMap) {
    logger.log_info(format!("{} connection-handler", context.slug), format!("Connection received"));
    let response =
        match parse_request(&stream, &logger, &context) {
            Ok(request) => {
                match handle_request(request, &logger, &context, &route_map) {
                    Ok(response) => response,
                    Err(e) => {
                        logger.log_error(format!("{} connection-handler", context.slug), format!("Error in response handler: {}", e));
                        handle_500()
                    }
                }
            }
            Err(e) => {
                logger.log_error(format!("{} connection-handler", context.slug), format!("Error parsing http request: {}", e));
                handle_400()
            }
        };

    handle_response(&stream, response);
}

fn parse_request(mut stream: &TcpStream, logger: &Logger, context: &ConnectionContext) -> Result<HttpRequest, &'static str> {
    let mut buffer = [0; 4096];
    let mut body: Vec<u8> = Vec::new();
    logger.log_info(format!("{} http-parser", context.slug), format!("Parsing header."));
    stream.read(&mut buffer).unwrap();
    logger.log_info(format!("{} http-parser", context.slug), format!("Read to buffer."));
    let (header, body_start_index) = HttpRequestHeader::create_from_buffer(buffer)?;
    let body = match (header.content_length > 0, body_start_index + header.content_length as usize > 4096) {
        // Short cut -> content length is 0 so no body
        (false, _) => {
            None
        }
        // If the body_start_index + content length 
        // the request of the body is bigger than the buffer and more reads needed
        (true, true) => {
            // TODO handle!
            None
        }
        // If the body_start_index + content length < 2048,
        // the body is in the initial buffer and no more reading is needed.
        (true, false) => {
            let end = body_start_index + header.content_length as usize;

            let body = buffer[body_start_index..end].to_vec();

            Some(body)
        }
    };

    HttpRequest::create(header, body)
}

fn handle_request(request: HttpRequest, logger: &Logger, context: &ConnectionContext, route_map: &RouteMap) -> Result<HttpResponse, &'static str> {
    logger.log_info(format!("{} request-handler", context.slug), format!("Handling request for {}", request.header.route));
    
    route_map.handle(request)
}

fn handle_response(mut stream: &TcpStream, mut response: HttpResponse) {
    match stream.write(&response.to_bytes()) {
        Ok(_) => {
            stream.flush();
        }
        Err(_) => {}
    }
}

/*
fn get_details() -> Output {
    Command::new("sh")
        .arg("-c")
        .arg("lscpu")
        .output()
        .expect("failed to execute process")
}
*/

fn handle_500() -> HttpResponse {
    let body = " { \"message\": \"Server error\"}".as_bytes().to_vec();

    HttpResponse::create(500, String::from("application/json"), Some(body))
}

fn handle_400() -> HttpResponse {
    let body = " { \"message\": \"Bad request\"}".as_bytes().to_vec();

    HttpResponse::create(400, String::from("application/json"), Some(body))
}

fn handle_404() -> HttpResponse {
    let body = " { \"message\": \"Not found\"}".as_bytes().to_vec();

    HttpResponse::create(404, String::from("application/json"), Some(body))
}