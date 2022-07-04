use crate::logging::common::{ConsoleColor, LogItem, LogItemType};

use chrono::UTC;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;

pub struct Logger {
    name: String,
    sender: Sender<LogItem>,
}

pub struct Log {
    handler: JoinHandle<()>,
    sender: Sender<LogItem>,
}

impl Logger {
    pub fn create(name: String, sender: Sender<LogItem>) -> Logger {
        Logger { name, sender }
    }

    pub fn create_from(&self, name: String) -> Logger {
        Logger {
            name,
            sender: self.sender.clone(),
        }
    }

    pub fn log(&self, item: LogItem) -> Result<(), &'static str> {
        match self.sender.send(item) {
            Ok(_) => Ok(()),
            Err(_) => Err("Could not write to log."),
        }
    }

    pub fn log_info(&self, message: String) -> Result<(), &'static str> {
        self.log(LogItem::info(self.name.clone(), message))
    }

    pub fn log_success(&self, message: String) -> Result<(), &'static str> {
        self.log(LogItem::success(self.name.clone(), message))
    }

    pub fn log_error(&self, message: String) -> Result<(), &'static str> {
        self.log(LogItem::error(self.name.clone(), message))
    }

    pub fn log_warning(&self, message: String) -> Result<(), &'static str> {
        self.log(LogItem::warning(self.name.clone(), message))
    }

    pub fn log_debug(&self, message: String) -> Result<(), &'static str> {
        self.log(LogItem::debug(self.name.clone(), message))
    }
}

impl Log {
    pub fn start() -> Result<Log, &'static str> {
        let (sender, receiver) = mpsc::channel::<LogItem>();

        let _ = sender.send(LogItem::info(
            "Logger".to_string(),
            "Starting log".to_string(),
        ));

        let handler = thread::spawn(move || loop {
            let item = receiver.recv().unwrap();
            Log::print(item);
        });

        let _ = sender.send(LogItem::success(
            "Log".to_string(),
            "Log started".to_string(),
        ));

        Ok(Log { handler, sender })
    }

    pub fn get_logger(&self, name: String) -> Logger {
        Logger {
            name,
            sender: self.sender.clone(),
        }
    }

    fn print(item: LogItem) {
        let (color, name) = match item.item_type {
            LogItemType::Information => (ConsoleColor::WhiteBright, "info  "), //{}
            LogItemType::Success => (ConsoleColor::Green, "ok    "),
            LogItemType::Error => (ConsoleColor::Red, "error "),
            LogItemType::Warning => (ConsoleColor::Yellow, "warn  "),
            LogItemType::Trace => (ConsoleColor::BlackBright, "debug "),
            LogItemType::Debug => (ConsoleColor::Magenta, "trace "),
        };

        color.set_foreground();
        println!(
            "[{} {}] {} - {}",
            UTC::now().format("%F %H:%M:%S%.3f"),
            name,
            item.from,
            item.message
        );
        ConsoleColor::reset();
    }
}
