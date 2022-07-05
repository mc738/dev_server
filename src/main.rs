use std::sync::mpsc;

use crate::files::FileWatcher;
use crate::http::server::Server;
use crate::logging::logger::Log;
use crate::messaging::MessageHub;
pub mod files;
pub mod http;
pub mod logging;
pub mod messaging;
pub mod watcher;
pub mod ws;

fn main() {
    let log = Log::start().unwrap();

    let (not_tx, not_rx) = mpsc::channel();
    let (sub_tx, sub_rx) = mpsc::channel();

    let base_path = "/home/max/Projects/sites/test".to_string();

    let watch = FileWatcher::start(not_tx, base_path.clone(), &log);

    let message_hub = MessageHub::start(sub_rx, not_rx, &log);

    let server = Server::start("127.0.0.1:8080".to_string(), &log, sub_tx, base_path);

    loop {}

    println!("Hello, world!");
}
