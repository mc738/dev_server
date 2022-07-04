use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::logging::logger::Log;

#[derive(Clone)]
pub enum Notification {
    FileUpdated(String),
}

pub struct Subscription {
    sender: Sender<Notification>,
}

pub struct MessageHub {
    thread: JoinHandle<()>,
}

impl MessageHub {
    pub fn start(
        receiver: Receiver<Subscription>,
        notifications: Receiver<Notification>,
        log: &Log,
    ) -> MessageHub {
        let mut subscribers: Vec<Sender<Notification>> = Vec::new();
        let logger = log.get_logger("message_hub".to_string());

        let thread = thread::spawn(move || loop {
            // Check for new subscribers
            match receiver.try_recv() {
                Ok(sub) => {
                    logger.log_info("Subscription received".to_string());
                    subscribers.push(sub.sender);
                }
                Err(_) => {}
            };

            match notifications.recv_timeout(Duration::from_secs(1)) {
                Ok(notification) => {
                    logger.log_info("Notification received".to_string());
                    for sub in &subscribers {
                        sub.send(notification.clone()).unwrap();
                    }
                }
                Err(_) => {}
            };

            // Send notifications to subscribers
        });

        MessageHub { thread }
    }
}

impl Subscription {
    pub fn new(sender: Sender<Notification>) -> Subscription {
        Subscription { sender }
    }
}
