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
                            notify::DebouncedEvent::Create(e) => send_message(&sender, e),
                            notify::DebouncedEvent::Write(e) => send_message(&sender, e),
                            notify::DebouncedEvent::Chmod(_) => {}
                            notify::DebouncedEvent::Remove(e) => send_message(&sender, e),
                            notify::DebouncedEvent::Rename(_, _) => {}
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

fn send_message(sender: &Sender<Notification>, path_buf: PathBuf) {
    println!("Hmmm {:?}", path_buf);
    let r = sender.send(Notification::FileUpdated(
        path_buf.to_str().unwrap().to_string(),
    ));

    match r {
        Ok(_) => {}
        Err(e) => println!("{}", e),
    };
}
