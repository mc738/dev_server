use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::logging::logger::Log;

#[derive(Clone)]
pub enum Notification {
    FileCreated(String),
    FileUpdated(String),
    FileRemoved(String),
    FileRenamed(String, String),
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
        let mut dead_subs: Vec<usize> = Vec::new();
        let logger = log.get_logger("message_hub".to_string());

        let thread = thread::spawn(move || loop {
            // Check for new subscribers
            match receiver.try_recv() {
                Ok(sub) => {
                    logger
                        .log_info("Subscription received".to_string())
                        .unwrap();
                    subscribers.push(sub.sender);
                }
                Err(_) => {}
            };

            match notifications.recv_timeout(Duration::from_secs(1)) {
                Ok(notification) => {
                    logger
                        .log_info("Notification received".to_string())
                        .unwrap();
                    for (i, sub) in &mut subscribers.iter().enumerate() {
                        match sub.send(notification.clone()) {
                            Ok(_) => logger
                                .log_info("Notification sent to subscriber".to_string())
                                .unwrap(),
                            Err(e) => {
                                // Subscriber pipe broken. Drop subscriber.
                                logger.log_warning(format!("Failure sending to subscriber, subscription to be dropped. Error: {}", e)).unwrap();
                                dead_subs.push(i);
                            }
                        };
                    }

                    if dead_subs.len() > 0 {
                        // Revserve so subs with a highest index are removed first.
                        // Example:
                        // 0, 1*, 2, 3* (* = remove).
                        // 3 will be removed leaving 0, 1, 2.
                        // Then 1 will be removed. To avoid calculating new next etc.
                        dead_subs.reverse();

                        for i in &dead_subs {
                            subscribers.remove(*i);
                        }

                        dead_subs.clear();
                    };
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
