use std::collections::HashMap;
use std::fs;
use std::process::Output;
use std::sync::mpsc::Sender;
use regex::Regex;
use serde_json::{Map, Value};
use crate::commands::format_output;
use crate::http::HttpResponse;
use crate::orchestration::{Job, JobCommand, JobHandler};
use crate::routing::{Route, RouteHandler, RouteMap};

pub struct Configuration {
    pub name: String,
    pub address: String,
    pub routes: RouteMap,
}

#[derive(Debug)]
pub struct JobsConfiguration {
    jobs: HashMap<String, JobConfiguration>,
}

#[derive(Debug)]
pub struct JobConfiguration {
    pub name: String,
    pub actions: Vec<ActionType>,
}

#[derive(Debug)]
pub enum ActionType {
    Command(CommandActionType),
    Test(TestActionType),
}

#[derive(Debug)]
pub struct CommandActionType {
    pub command_name: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct TestActionType {
    pub wait_time: i64,
}

impl Configuration {
    pub fn load(path: String, job_handler: Sender<JobCommand>) -> Result<Configuration, &'static str> {
        load_config(path, job_handler)
    }
}

impl JobsConfiguration {
    pub fn load(path: String) -> Result<JobsConfiguration, &'static str> {
        load_jobs_config(path)
    }
    
    pub fn get_job(&self, name: &str) -> Option<&JobConfiguration> {
        self.jobs.get(name)
    }
}

impl ActionType {
    pub fn create_command(name: String, args: Vec<String>) -> ActionType {
        ActionType::Command(CommandActionType { command_name: name, args })
    }

    pub fn create_test(wait_time: i64) -> ActionType {
        ActionType::Test(TestActionType { wait_time })
    }
}

fn get_string(value: &Value) -> String {
    match value.as_str() {
        None => String::new(),
        Some(v) => v.to_string()
    }
}

fn load_config(path: String, job_handler: Sender<JobCommand>) -> Result<Configuration, &'static str> {
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

fn create_route_map(mut routes_array: Value, job_handler: Sender<JobCommand>) -> Result<RouteMap, &'static str> {
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

fn load_jobs_config(path: String) -> Result<JobsConfiguration, &'static str> {
    let config_json = fs::read_to_string(path).expect("Fail");
    let config_json = config_json.trim_start_matches('﻿');
    let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(&config_json.clone());

    println!("{}", config_json);
    match parse_result {
        Ok(json) => {
            match json.get("jobs") {
                None => Err("Jobs value not found."),
                Some(jobs_arr) => {
                    println!("Jobs found");
                    match jobs_arr.as_array() {
                        Some(jobs) => {
                            let mut jobs_map: HashMap<String, JobConfiguration> = HashMap::new();

                            let _ =
                                jobs
                                    .iter()
                                    .map(|job|
                                        {
                                            println!("Creating job.");
                                            let r = create_job_config(&mut job.clone()).unwrap();
                                            jobs_map.insert(r.name.clone(), r);
                                        }
                                    ).collect::<()>();
                            //.collect::<Vec<JobConfiguration>>()
                            //.iter()
                            //.map(|job| { ; 1})
                            //.collect<i32>();

                            Ok(JobsConfiguration { jobs: jobs_map })
                        }
                        None => Err("Jobs value is not an array.")
                    }
                }
            }


            //let name = get_string(&json["name"]);
            //let address = get_string(&json["address"]);
            //let routes_obj = json["routes"].clone();
            //let routes = create_route_map(routes_obj, job_handler)?;
            //Ok(Configuration { name, address, routes })
            //Err("TODO")
        }
        Err(e) => {
            println!("Error parsing config.json: {}", e);
            Err("Error parsing config")
        }
    }
}

fn create_job_config(mut job_obj: &mut Value) -> Result<JobConfiguration, &'static str> {
    match job_obj.as_object_mut() {
        None => Err("Job value is not and object"),
        Some(jo) => {
            let get_values = (jo.get("name"), jo.get("actions"));

            match get_values {
                (Some(name_value), Some(action_value)) => {
                    match action_value.as_array() {
                        Some(av) => {
                            let name = get_string(name_value);

                            let actions =
                                av.iter()
                                    .map(|a| create_action(a).unwrap())
                                    .collect();

                            Ok(JobConfiguration { name, actions })
                        }
                        None => Err("Actions value is not an array.")
                    }
                }
                (None, _) => Err("Missing name value"),
                (_, None) => Err("Missing type value"),
            }
        }
    }
}

fn create_action(mut action_obj: &Value) -> Result<ActionType, &'static str> {
    match action_obj.clone().as_object_mut() {
        None => Err("Action value is not and object"),
        Some(ao) => {
            let get_values = (ao.get("name"), ao.get("type"));

            match get_values {
                (Some(name_value), Some(type_value)) => {
                    let name = get_string(name_value);
                    let type_name = get_string(type_value).as_str();

                    match get_string(type_value).as_str() {
                        "command" => {
                            let command_values = (ao.get("command_name"), ao.get("args"));
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
                                    Ok(ActionType::create_command(
                                        get_string(name),
                                        argv))
                                }
                                (None, _) => Err("Missing command name"),
                                (_, None) => Err("Missing args")
                            }
                        }
                        "test" => {
                            match ao.get("wait_time") {
                                None => Err("Missing wait time value."),
                                Some(wtv) => {
                                    match wtv.as_i64() {
                                        None => Err("Test wait time is not i64."),
                                        Some(v) => Ok(ActionType::create_test(v))
                                    }
                                }
                            }
                        }
                        _ => Err("Unknown job type.")
                    }
                }
                (None, _) => Err("Missing name value"),
                (_, None) => Err("Missing type value"),
            }
        }
    }
}