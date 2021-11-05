use std::{thread, time};
use std::collections::HashMap;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::thread::JoinHandle;
use uuid::Uuid;
use crate::logging::logging::{Log, Logger};

pub struct Orchestrator {
    workers: WorkerPool,
    logger: Logger,
    handler: JoinHandle<()>
}

#[derive(Clone)]
pub struct Aggregator {
    sender: Sender<AggregatorMessage>
}

enum AggregatorMessage {
    NewJobSet(Uuid),
    ProgressReport(),
    CompletedJob(Uuid)
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

pub type JobHandler = Box<dyn FnOnce(Uuid) -> String + Send + 'static>;

pub struct Job {
    id: Uuid,
    handler: JobHandler
}

//pub type Job = 

impl Orchestrator {
    pub fn run(receiver: Receiver<JobHandler>, aggregator: Aggregator, logger: Logger) {
        //let (sender , receiver) : (Sender<Job>, Receiver<Job>) = mpsc::channel();

        let workers = WorkerPool::new(4, aggregator.clone(), logger.clone());
        
        loop {
            let job = receiver.recv().unwrap();
            logger.log_info(format!("orch"), format!("Job received."));
            // Send new job to aggregator.
            let id = Uuid::new_v4();
            logger.log_info(format!("orch"), format!("Job received. Assigned id: {}", id));
            aggregator.send_jobs(id);
            workers.execute(id, job)
        }
    }
}

impl Aggregator {
    pub fn start(logger: Logger) -> Aggregator {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(||{
            aggregating_handler(receiver, logger);
        });
        
        Aggregator { sender }
    }
    
    pub fn send_jobs(&self, id: Uuid) {
        self.sender.send(AggregatorMessage::NewJobSet(id));
    }
    
    pub fn get_progress(&self) {
        self.sender.send(AggregatorMessage::ProgressReport());
    }
    
    pub fn complete_job(&self, id: Uuid) {
        self.sender.send(AggregatorMessage::CompletedJob(id));
    }
}

fn aggregating_handler(receiver: Receiver<AggregatorMessage>, logger: Logger) {
    let mut jobs: HashMap<Uuid, _> = HashMap::new();

    logger.log_info("aggregator".to_string(),"Aggregator running.".to_string());
    loop {
        logger.log_debug("aggregator".to_string(),"Checking for messages.".to_string());
        match receiver.try_recv() {
            Ok(msg) => {
                match msg {
                    AggregatorMessage::NewJobSet(id) => {
                        logger.log_info("aggregator".to_string(), format!("New job {} received.", id));
                        jobs.insert(id, ());
                        logger.log_info("aggregator".to_string(), format!("Outstanding jobs: {}", jobs.len()));
                    }
                    AggregatorMessage::ProgressReport() => {}
                    AggregatorMessage::CompletedJob(id) => {
                        logger.log_success("aggregator".to_string(), format!("Job {} complete.", id));
                        jobs.remove(&id);
                        logger.log_info("aggregator".to_string(), format!("Outstanding jobs: {}", jobs.len()));
                    }
                }
            }
            Err(e) => {
                logger.log_debug("aggregator".to_string(), format!("No messages received. Reason: {}", e));
                let wait_time = time::Duration::from_millis(1000);
                thread::sleep(wait_time);
            }
        }
    }
}

impl WorkerPool {
    pub fn new(size: usize, aggregator: Aggregator, logger: Logger) -> WorkerPool {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver), aggregator.clone(), logger.clone()));
        }

        WorkerPool { workers, sender }
    }

    pub fn execute<F>(&self, id: Uuid, f: F)
        where
            F: FnOnce(Uuid) -> String + Send + 'static,
    {
        let job = 
            Job {
                id,
                handler: Box::new(f)
            };
            
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>, aggregator: Aggregator, logger: Logger) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            //let job_id = Uuid::new_v4();
            logger.log_info(format!("worker_{}", id), format!("Job received. id: {}", job.id));
            //println!("Worker {} got a job. Executing", id);
            //println!("Handled by {}", id);
            let r = (job.handler)(job.id);
            logger.log_success(format!("worker_{}", id), format!("Job {} complete.", job.id));
            aggregator.complete_job(job.id);
        });

        Worker { id, thread }
    }
}