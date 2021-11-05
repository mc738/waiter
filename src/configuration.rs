use std::fs;
use std::process::Output;
use std::sync::mpsc::Sender;
use regex::Regex;
use serde_json::Value;
use crate::commands::format_output;
use crate::http::HttpResponse;
use crate::orchestration::{Job, JobHandler};
use crate::routing::{Route, RouteHandler, RouteMap};

pub struct Configuration {
    pub name: String,
    pub address: String,
    pub routes: RouteMap,
}

impl Configuration {
    pub fn load(path: String, job_handler: Sender<JobHandler>) -> Result<Configuration, &'static str> {
        load_config(path, job_handler)
    }
}

fn get_string(value: &Value) -> String {
    match value.as_str() {
        None => String::new(),
        Some(v) => v.to_string()
    }
}

fn load_config(path: String, job_handler: Sender<JobHandler>) -> Result<Configuration, &'static str> {
    let config_json = fs::read_to_string(path).expect("Fail");
    let config_json = config_json.trim_start_matches('﻿');
    let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(&config_json.clone());

    match parse_result {
        Ok(json) => {
            let name = get_string(&json["name"]);
            let address = get_string(&json["address"]);
            let routes_obj = json["routes"].clone();
            let routes = create_route_map(routes_obj, job_handler)?;
            Ok(Configuration { name, address, routes })
        }
        Err(e) => {
            println!("Error parsing config.json: {}", e);
            Err("Error parsing config")
        }
    }
}

fn create_route_map(mut routes_array: Value, job_handler: Sender<JobHandler>) -> Result<RouteMap, &'static str> {
    let routes =
        match routes_array.as_array() {
            None => Err("Routes value is not an array"),
            Some(ra) => {
                ra.iter().map(|mut ro| create_route_from_value(&mut ro.clone())).collect()
            }
        }?;
    Ok(RouteMap::new(job_handler, routes))
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
                            "job" => {
                                let command_values = (vm.get("name"), vm.get("args"));
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
                                        Ok(RouteHandler::create_job(
                                            get_string(name),
                                            argv))
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
    match format_output(output) {
        Ok(json) => {
            HttpResponse::create(200, String::from("application/json"), Some(Vec::from(json.as_bytes())))
        }
        Err(e) => {
            HttpResponse::create(500, String::from("text/plain"), Some(Vec::from(e.as_bytes())))

        }
    }
}