use std::{thread, time};
use std::collections::HashMap;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::thread::JoinHandle;
use uuid::Uuid;
use crate::commands::{format_output, run_command, run_command_static};
use crate::configuration::{ActionType, JobConfiguration, JobsConfiguration};
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

pub struct JobCommand {
    pub(crate) name: String,
    pub(crate) reply_channel: Sender<Result<Uuid, &'static str>>
}

//pub type Job = 

impl Orchestrator {
    pub fn run(receiver: Receiver<JobCommand>, aggregator: Aggregator, jobs_config: JobsConfiguration, logger: Logger) {
        //let (sender , receiver) : (Sender<Job>, Receiver<Job>) = mpsc::channel();

        let workers = WorkerPool::new(4, aggregator.clone(), logger.clone());
        
        loop {
            
            let job_command = receiver.recv().unwrap();
            logger.log_info(format!("orch"), format!("Job command received."));
            // Get the job command.
            match jobs_config.get_job(job_command.name.as_str()) {
                None => {
                    logger.log_error("orch".to_string(), format!("Job `{}` not found.", job_command.name));
                    job_command.reply_channel.send(Err("Job not found."));
                }
                Some(jc) => {
                    let id = Uuid::new_v4();
                    
                    let job_ids: Vec<Uuid> =
                        jc.actions
                            .iter()
                            .map(|a|{
                              let j = create_job_handler(a);
                                workers.execute(j.id, j.handler);
                                aggregator.send_jobs(id);
                                j.id
                            })
                            .collect();
                    
                    
                    // Create the job handler(s) for actions.
                    logger.log_info(format!("orch"), format!("Job received. Assigned id: {}", id));
                    aggregator.send_jobs(id);
                    //workers.execute(id, job)        
                }
            }
            
            
            
            // Get actions.
            
            // Send actions to workers.
            
            // Send job set to aggregator.
            
            // Send back job_id.
            
            // Send new job to aggregator.
            
        }
    }
}

fn create_job_handler(action: &ActionType) -> Job {
    let id= Uuid::new_v4();
    
    let job_handler =
        match action {
            ActionType::Command(ac) => {
                let name = &ac.command_name.clone();
                let args = &ac.args.clone();
                execute_command(name.clone(), args.clone())   
            }
            ActionType::Test(tc) => {
                test_job(tc.wait_time.unsigned_abs())   
            }
        };
    
    Job { id, handler: job_handler }
    
}

fn execute_command(name: String, args: Vec<String>) -> JobHandler {
    Box::new(|id: Uuid|{
        let output= run_command_static("".to_string(), Vec::new()).unwrap();
        //let format
        format_output(output).unwrap()
    })
}

fn test_job(wait_time: u64) -> JobHandler {
    let handler= (move |id: Uuid|{
        println!("*** TEST JOB - Job {} received. Simulating work...", id);
        let wait_time = time::Duration::from_millis(wait_time);
        thread::sleep(wait_time);
        println!("*** TEST JOB - Job {} completed.", id);
        format!("Job reference: {}", id)
    });
    Box::new(handler)
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