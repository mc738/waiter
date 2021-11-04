mod logging;
mod connection_pool;
mod http;
mod server;
mod orchestration;
mod routing;

use std::net::{TcpListener, TcpStream};
use std::fs;
use std::fs::read_to_string;
use std::process::{Command, Output};
use crate::connection_pool::ConnectionPool;
use crate::logging::logging::{Log, Logger};
use crate::server::Server;
use regex::{Error, Regex};
use serde_json::{Map, Value};
use crate::http::HttpResponse;
use crate::routing::{Route, RouteHandler, RouteMap};

fn main() {
    match load_config("config.json".to_string()) {
        Ok(config) => {
            println!("{:?}", config);
            Server::start(config)
        }
        Err(e) => {
            println!("Error loading config: {}", e)
        }
    }
}

fn get_string(value: &Value) -> String {
    match value.as_str() {
        None => String::new(),
        Some(v) => v.to_string()
    }
}

fn load_config(path: String) -> Result<RouteMap, &'static str> {
    let config_json = fs::read_to_string(path).expect("Fail");
    let config_json = config_json.trim_start_matches('﻿');
    println!("Json: {}", config_json);
    let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(&config_json.clone());

    match parse_result {
        Ok(json) => {
            //let json = serde_json::from_str(&config_json);
            let name = json["name"].clone();
            let address = json["address"].clone();
            let routes = json["routes"].clone();
            //)
            create_route_map(routes)
        }
        Err(e) => {
            println!("Error parsing config.json: {}", e);
            Err("Error parsing config")
        }
    }
}

fn create_route_map(mut routes_array: Value) -> Result<RouteMap, &'static str> {
    let routes =
        match routes_array.as_array() {
            None => Err("Routes value is not an array"),
            Some(ra) => {
                ra.iter().map(|mut ro| create_route_from_value(&mut ro.clone())).collect()
            }
        }?;
    Ok(RouteMap::new(routes))
}

fn create_route_from_value(route_obj: &mut Value) -> Result<Route, &'static str> {
    match route_obj.as_object_mut() {
        None => Err("Json value is not a object."),
        Some(vm) => {
            let get_values = (vm.get("regex"), vm.get("type"));

            match get_values {
                (Some(regex), Some(route_type)) => {
                    let route_handler =
                        match get_string(route_type).as_str() {
                            "static" => {
                                let static_values = (vm.get("content_path"), vm.get("content_type"));
                                match static_values {
                                    (Some(content_path), Some(content_type)) => {
                                        Ok(RouteHandler::create_static(get_string(content_path), get_string(content_type)))
                                    }
                                    (None, _) => Err("Missing content path"),
                                    (_, None) => Err("Missing content type")
                                }
                            }
                            "command" => {
                                let command_values = (vm.get("command_name"), vm.get("args"));
                                match command_values {
                                    (Some(name), Some(args)) => {
                                        let argv =
                                            match args.as_array() {
                                                None => vec![],
                                                Some(argv) => {
                                                    argv
                                                        .iter()
                                                        .map(|s| {
                                                            match s.as_str() {
                                                                None => String::new(),
                                                                Some(v) => v.to_string()
                                                            }
                                                        })
                                                        .collect()
                                                }
                                            };
                                        Ok(RouteHandler::create_command(
                                            get_string(name),
                                            argv,
                                            handle_command))
                                    }
                                    (None, _) => Err("Missing command name"),
                                    (_, None) => Err("Missing args")
                                }
                            }
                            _ => {
                                println!("Type: {}", route_type.to_string().as_str());
                                Err("Unknown route type")
                            }
                        };

                    match route_handler {
                        Ok(handler) =>
                            {
                                let r = Regex::new(&*get_string(regex)).unwrap();
                                Ok(Route::new(r, handler))
                            }
                        Err(e) => Err(e)
                    }
                }
                (None, _) => Err("Missing route regex"),
                (_, None) => Err("Missing route type")
            }
        }
    }
}

fn handle_command(output: Output) -> HttpResponse {
    HttpResponse::create(200, String::from("text/html"), Some(output.stdout))
}