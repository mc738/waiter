mod logging;
mod connection_pool;
mod http;
mod server;
mod orchestration;
mod routing;
mod agents;
mod configuration;
mod commands;

use std::net::{TcpListener, TcpStream};
use std::{fs, thread, time};
use std::fs::read_to_string;
use std::process::{Command, Output};
use std::sync::mpsc::channel;
use std::thread::Thread;
use crate::connection_pool::ConnectionPool;
use crate::logging::logging::{Log, Logger};
use crate::server::Server;
use regex::{Error, Regex};
use serde_json::{Map, Value};
use crate::agents::{Agent, MessageType};
use crate::commands::{format_output, run_command, run_command_static};
use crate::http::HttpResponse;
use crate::routing::{Route, RouteHandler, RouteMap};
use crate::configuration::*;
use crate::orchestration::{Aggregator, Orchestrator};

fn main() {
    
    let jobs_config = JobsConfiguration::load("jobs.json".to_string()).unwrap();
    
    let log = Log::create().unwrap();
    let logger = log.get_logger();
    let (job_sender, job_receiver) = channel();

    let orch_logger = log.get_logger();
    let aggregator = Aggregator::start(log.get_logger());
    let orch_agg = aggregator.clone();

    let _ = thread::spawn(|| {
        Orchestrator::run(job_receiver, orch_agg, jobs_config, orch_logger)
    });
    
    match Configuration::load("config.json".to_string(), job_sender.clone()) {
        Ok(config) => {
            //println!("{:?}", jobs_config);
            Server::start(config, log.get_logger())
        }
        Err(e) => {
            println!("Error loading config: {}", e)
        }
    }
}