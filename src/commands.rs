use std::str;
use std::process::{Command, ExitStatus, Output};
use std::str::Utf8Error;
use serde_json::Value;

pub fn run_command(name: &String, args: &Vec<String>) -> Result<Output, &'static str> {
    let mut command = Command::new(name);
    let output =
        args
            .iter()
            .fold(&mut command, |mut acc, arg| acc.arg(arg))
            .output();

    match output {
        Ok(output) => Ok(output),
        Err(e) => {
            Err("Error running command.")
        }
    }
}

pub fn format_output(output: Output) -> Result<String, &'static str> {
    match output.status.success() {
        true => {
            match str::from_utf8(&*output.stdout) {
                Ok(output_text) => {
                    let lines: Vec<Value> = 
                        output_text
                            .split("\n")
                            .map(|l| Value::String(l.to_string()))
                            .collect();
                    
                    
                    let arr = Value::Array(lines);
                    
                    Ok(arr.to_string())
                }
                Err(_) => Err("Failed to read output.")
            }
        }
        false => Err("Process failed.")
    }
}