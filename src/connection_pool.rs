use std::thread;
use std::sync::{Arc, mpsc, Mutex};
use crate::logging::logging::Logger;
use crate::orchestration::{Job, JobHandler};
use crate::routing::RouteMap;

pub struct ConnectionPool {
    workers: Vec<ConnectionHandler>,
    sender: mpsc::Sender<Connection>,
}

type Connection = Box<dyn FnOnce() + Send + 'static>;

impl ConnectionPool {
    pub fn new(size: usize, logger: Logger) -> ConnectionPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(ConnectionHandler::new(id, Arc::clone(&receiver), logger.clone()));
        }

        ConnectionPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

struct ConnectionHandler {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl ConnectionHandler {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Connection>>>, logger: Logger) -> ConnectionHandler {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            logger.log_info(format!("connection_handler_{}", id), format!("Connection received."));
            job();
        });

        ConnectionHandler { id, thread }
    }
}