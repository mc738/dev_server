use std::{
    path::PathBuf,
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use notify::{watcher, RecursiveMode, Watcher};

use crate::{logging::logger::Log, messaging::Notification};

pub struct FileWatcher {
    thread: JoinHandle<()>,
}

impl FileWatcher {
    /// Start the file watcher. This will return a FileWatcher with the related thread's
    /// JoinHandle.
    ///
    /// # Panics
    ///
    /// Panics if an DebouncedEvent::Error is returned.
    pub fn start(sender: Sender<Notification>, base_path: String, log: &Log) -> FileWatcher {
        let (tx, rx) = mpsc::channel();
        let logger = log.get_logger("file_watcher".to_string());

        let thread = thread::spawn(move || {
            let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

            watcher.watch(base_path, RecursiveMode::Recursive).unwrap();

            loop {
                match rx.recv() {
                    Ok(event) => {
                        match event {
                            notify::DebouncedEvent::NoticeWrite(_) => {}
                            notify::DebouncedEvent::NoticeRemove(_) => {}
                            notify::DebouncedEvent::Create(e) => send_message(
                                &sender,
                                Notification::FileCreated(path_buf_to_string(e)),
                            ),
                            notify::DebouncedEvent::Write(e) => send_message(
                                &sender,
                                Notification::FileUpdated(path_buf_to_string(e)),
                            ),
                            notify::DebouncedEvent::Chmod(_) => {}
                            notify::DebouncedEvent::Remove(e) => send_message(
                                &sender,
                                Notification::FileRemoved(path_buf_to_string(e)),
                            ),
                            notify::DebouncedEvent::Rename(o, n) => send_message(
                                &sender,
                                Notification::FileRenamed(
                                    path_buf_to_string(o),
                                    path_buf_to_string(n),
                                ),
                            ),
                            notify::DebouncedEvent::Rescan => {}
                            notify::DebouncedEvent::Error(_, _) => todo!(),
                        };
                    }
                    Err(_) => {
                        logger.log_error("Watcher error.".to_string()).unwrap();
                    }
                }
            }
        });

        FileWatcher { thread }
    }
}

/// Convert a PathBuf to a String.
///
/// # Panics
///
/// Panics if the PathBuf contains non-UTF8 characters.
fn path_buf_to_string(path_buf: PathBuf) -> String {
    path_buf.to_str().unwrap().to_string()
}

/// Send a notification message.
fn send_message(sender: &Sender<Notification>, notification: Notification) {
    let r = sender.send(notification);

    match r {
        Ok(_) => {}
        Err(e) => println!("{}", e),
    };
}
