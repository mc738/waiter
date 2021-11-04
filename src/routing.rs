use std::fs;
use std::process::Command;
use regex::Regex;
use crate::http::{HttpRequest, HttpResponse};

#[derive(Clone)]
pub struct StaticRoute {
    content_path: String,
    content_type: String,
}

#[derive(Clone)]
pub struct CommandRoute {
    command_name: String,
    args: Vec<String>,
}

#[derive(Clone)]
pub enum RouteHandler {
    Static(StaticRoute),
    Command(CommandRoute),
}

impl RouteHandler {
    pub fn create_static(content_path: String, content_type: String) -> RouteHandler {
        RouteHandler::Static(StaticRoute { content_path, content_type })
    }
    
    pub fn create_command(command_name: String, args: Vec<String>) -> RouteHandler {
        RouteHandler::Command(CommandRoute { command_name, args })
    }
    
    pub fn handle(&self, request: HttpRequest) -> Result<HttpResponse, &'static str> {
        match self {
            RouteHandler::Static(sr) => {
                let body = fs::read(&sr.content_path).unwrap();
                let response = HttpResponse::create(200, String::from(&sr.content_type), Some(body));
                Ok(response)
            }
            RouteHandler::Command(cr) => {
                let mut command = Command::new(&cr.command_name);
                let output =
                    cr.args
                        .iter()
                        .fold(&mut command, |mut acc, arg| acc.arg(arg))
                        .output()
                        .unwrap();
                
                let response = HttpResponse::create(200, String::from("text/html"), Some(output.stdout));
                Ok(response)
            }
        }
    }
}

#[derive(Clone)]
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
pub struct RouteMap {
    pub routes: Vec<Route>,
}

impl RouteMap {
    
    pub fn new(routes: Vec<Route>) -> RouteMap {
        RouteMap { routes }
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
            Some(r) => r.handler.handle(request)
        }
    }
}