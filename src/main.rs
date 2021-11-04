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
use regex::Regex;
use crate::routing::{Route, RouteHandler, RouteMap};

fn main() {

    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(re.is_match("2014-01-01"));
    
    
    Server::start(create_routes());
}

fn create_routes() -> RouteMap {
    
    let args = vec![ format!("-c"), format!("lscpu") ];
    
    let home_handler = RouteHandler::create_static(format!("index.html"), format!("text/html"));
    let info_handler = RouteHandler::create_command(format!("sh"), args);
    let home = Route::new(Regex::new(r"(^/index$|^/$|^/home$)").unwrap(), home_handler);
    let info = Route::new(Regex::new(r"^/info$").unwrap(), info_handler);
    RouteMap::new(vec![ home, info ])
}