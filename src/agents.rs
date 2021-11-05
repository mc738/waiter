use std::sync::mpsc;
use std::{thread, time};

pub struct Agent<T> where T: Send + 'static {
    sender: mpsc::Sender<MessageType<T>>
}

pub struct MailBox<T: 'static + Send>  {
    sender: mpsc::Sender<MessageType<T>>
}

pub enum  MessageType<T> where T: Send + 'static {
    Post(T),
    PostAndReply(T, mpsc::Sender<T>)
}

impl<T: 'static + Send> Agent<T> {
    
    pub fn start(handler: Box<dyn Fn(MessageType<T>) -> () + Send + 'static>) -> Agent<T> {
        let (sender,receiver) = mpsc::channel();
        
        // start the agent thread.
        println!("Starting agent thread");
        let _ = thread::spawn(move || {
            loop {
                println!("Agent - waiting for value");
                let msg = receiver.recv().unwrap();
                handler(msg);
            }
        });
        
        Agent { sender }
    }
    
    pub fn get_mailbox(&self) -> MailBox<T> {
        MailBox { sender: self.sender.clone() }
    }
}

impl<T: 'static + Send> MailBox<T> {
    pub fn post(&self, value: T) {
        //println!("Mailbox - sending value: {}", value);
        self.sender.send(MessageType::Post(value)).unwrap()
    }
    
    pub fn post_and_reply(&self, value: T) -> T {
        //println!("Mailbox - sending value: {}", value);
        let (sender, reply) = mpsc::channel();
        self.sender.send(MessageType::PostAndReply(value, sender));
        reply.recv().unwrap()
    }
}