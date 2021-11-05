use std::{fs, thread, time};
use std::process::{Command, Output};
use std::sync::mpsc::Sender;
use regex::Regex;
use uuid::Uuid;
use crate::commands::run_command;
use crate::http::{HttpRequest, HttpResponse};
use crate::orchestration::{Job, JobHandler};

#[derive(Clone)]
#[derive(Debug)]
pub struct StaticRoute {
    content_path: String,
    content_type: String,
}

#[derive(Clone)]
#[derive(Debug)]
pub struct CommandRoute {
    command_name: String,
    args: Vec<String>,
    response_handler: fn(Output) -> HttpResponse
}

#[derive(Clone)]
#[derive(Debug)]
pub struct JobRoute {
    name: String,
    args: Vec<String>,
    //response_handler: fn(Output) -> HttpResponse
}

#[derive(Clone)]
#[derive(Debug)]
pub enum RouteHandler {
    Static(StaticRoute),
    Command(CommandRoute),
    Job(JobRoute)
}

impl RouteHandler {
    pub fn create_static(content_path: String, content_type: String) -> RouteHandler {
        RouteHandler::Static(StaticRoute { content_path, content_type })
    }
    
    pub fn create_command(command_name: String, args: Vec<String>, response_handler: fn(Output) -> HttpResponse) -> RouteHandler {
        RouteHandler::Command(CommandRoute { command_name, args, response_handler })
    }
    
    pub fn create_job(name: String, args: Vec<String>) -> RouteHandler {
        RouteHandler::Job(JobRoute { name, args })
    }
    
    pub fn handle(&self, job_handler: Sender<JobHandler>, request: HttpRequest) -> Result<HttpResponse, &'static str> {
        match self {
            RouteHandler::Static(sr) => {
                let body = fs::read(&sr.content_path).unwrap();
                let response = HttpResponse::create(200, String::from(&sr.content_type), Some(body));
                Ok(response)
            }
            RouteHandler::Command(cr) => {
                let output= run_command(&cr.command_name, &cr.args)?;
                //let format
                let response = (cr.response_handler)(output);
                Ok(response)
            }
            RouteHandler::Job(jr) => {
                job_handler.send(Box::new(test_job));
                let body = "Job queued".bytes().collect();
                let response = HttpResponse::create(201, "text/plain".to_string(), Some(body));
                Ok(response)
            }
        }
    }
}

fn test_job(id: Uuid) -> String {
    println!("*** TEST JOB - Job {} received. Simulating work...", id);
    let wait_time = time::Duration::from_millis(1000);
    thread::sleep(wait_time);
    println!("*** TEST JOB - Job {} completed.", id);
    format!("Job reference: {}", id)
}

#[derive(Clone)]
#[derive(Debug)]
pub struct Route {
    route_regex: Regex,
    handler: RouteHandler,
}

impl Route {
    pub fn new(route_regex: Regex, handler: RouteHandler) -> Route {
        Route { route_regex, handler }
    }

    pub fn is_match(&self, route: &String) -> bool {
        self.route_regex.is_match(route)
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct RouteMap {
    pub routes: Vec<Route>,
    job_handler: Sender<JobHandler>
}

impl RouteMap {
    
    pub fn new(job_handler: Sender<JobHandler>, routes: Vec<Route>) -> RouteMap {
        RouteMap { routes, job_handler }
    }
    
    pub fn handle(&self, request: HttpRequest) -> Result<HttpResponse, &'static str> {
        let route =
            self.routes
                .iter()
                .fold(None, |acc, r| match r.is_match(&request.header.route) {
                    true => Some(r),
                    false => acc
                });

        match route {
            None => Err("Route not found"),
            Some(r) => r.handler.handle(self.job_handler.clone(), request)
        }
    }
}