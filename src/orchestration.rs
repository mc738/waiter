use std::thread;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::thread::JoinHandle;
use crate::logging::logging::Logger;

pub struct Orchestrator {
    workers: WorkerPool,
    logger: Logger,
    handler: JoinHandle<()>
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

pub type Job = Box<dyn FnOnce() + Send + 'static>;

impl Orchestrator {
    pub fn run(receiver: Receiver<Job>, logger: Logger) {
        //let (sender , receiver) : (Sender<Job>, Receiver<Job>) = mpsc::channel();

        let workers = WorkerPool::new(4, logger.clone());
        
        loop {
            let job = receiver.recv().unwrap();
            logger.log_info(format!("orch"), format!("Job received."));
            workers.execute(job)
        }
        
        //let logger = logger.clone();
        //let handler = thread::spawn(move || loop {
        //});
        
        
        //Orchestrator { sender, workers, logger, handler }
        
    }
}


impl WorkerPool {
    pub fn new(size: usize, logger: Logger) -> WorkerPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver), logger.clone()));
        }

        WorkerPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>, logger: Logger) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            logger.log_info(format!("worker_{}", id), format!("Job received."));
            //println!("Worker {} got a job. Executing", id);
            //println!("Handled by {}", id);
            job();
        });

        Worker { id, thread }
    }
}