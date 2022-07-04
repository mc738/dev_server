use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    http::common::{HttpRequest, HttpResponse, HttpStatus},
    logging::logger::{Log, Logger},
    messaging::Subscription,
    ws,
};

pub(crate) struct Server {
    thread: JoinHandle<()>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct ConnectionPool {
    sender: Sender<Job>,
    workers: Vec<Worker>,
}

struct Worker {
    id: usize,
    thread: JoinHandle<()>,
}

impl Server {
    pub fn start(
        address: String,
        log: &Log,
        sub_sender: Sender<Subscription>,
    ) -> Result<Server, &'static str> {
        let logger = log.get_logger("server".to_string());
        let connection_pool = ConnectionPool::new(4);

        match TcpListener::bind(address) {
            Ok(listener) => {
                let thread = thread::spawn(move || loop {
                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => {
                                let request_logger = logger.create_from("connection".to_string());
                                let ss = sub_sender.clone();

                                connection_pool
                                    .execute(|| handle_connection(stream, request_logger, ss));
                            }
                            Err(_) => todo!(),
                        };
                    }
                });

                Ok(Server { thread })
            }
            Err(_) => Err("Could not start server."),
        }
    }
}

impl ConnectionPool {
    fn new(size: usize) -> ConnectionPool {
        let mut workers = Vec::with_capacity(size);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..size {
            let name = format!("worker_{}", id);
            workers.push(Worker::new(id, receiver.clone()));
        }

        ConnectionPool { sender, workers }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            job();
        });

        Worker { id, thread }
    }
}

fn handle_connection(mut stream: TcpStream, logger: Logger, sub_sender: Sender<Subscription>) {
    match HttpRequest::from_stream(&stream, &logger) {
        Ok(request) => match request.header.route.as_str() {
            "/ws/notify" => {
                logger
                    .log_info(format!("Update notification requested"))
                    .unwrap();
                handle_ws_connection(request, stream, sub_sender);
                // TODO handle ws.
                // Keep alive...
            }
            _ => match File::open(get_path(request.header.route.clone())) {
                Ok(_) => {}
                Err(_) => {
                    let mut response = HttpResponse::create(
                        HttpStatus::NotFound,
                        "text/plain".to_string(),
                        HashMap::new(),
                        None,
                    );

                    stream.write(&response.to_bytes()).unwrap();
                }
            },
        },
        Err(_) => todo!(),
    };
}

fn get_path(route: String) -> String {
    route
}

fn handle_ws_connection(
    mut request: HttpRequest,
    mut stream: TcpStream,
    sub_sender: Sender<Subscription>,
) {
    println!("WS connection");
    let mut b = request.to_bytes();

    println!("{}", String::from_utf8(b.to_vec()).unwrap());
    match request.header.headers.get("SEC-WEBSOCKET-KEY") {
        Some(key) => {
            println!("Key: {}", key);
            let ws_handshake = ws::handle_handshake(key);
            println!("Handshake: {}", ws_handshake);

            let mut addition_headers = HashMap::new();

            addition_headers.insert("Upgrade".to_string(), "websocket".to_string());
            addition_headers.insert("Connection".to_string(), "Upgrade".to_string());
            addition_headers.insert("Sec-WebSocket-Accept".to_string(), ws_handshake);
            addition_headers.insert("Sec-WebSocket-Version".to_string(), "13".to_string());

            let mut response = HttpResponse::create(
                HttpStatus::SwitchingProtocols,
                "text/plain".to_string(),
                addition_headers,
                None,
            );

            match stream.write(&mut response.to_bytes()) {
                Ok(_) => {
                    // Handle web socket connection
                    let (tx, rx) = mpsc::channel();

                    let thread = thread::spawn(move || loop {
                        sub_sender.send(Subscription::new(tx.clone()));

                        match rx.recv() {
                            Ok(notification) => {
                                let d = b"File updated";

                                stream
                                    .write(&ws::handle_write(&mut d.to_vec(), 12))
                                    .unwrap();
                            }
                            Err(_) => (),
                        };
                    });
                }
                Err(_) => todo!(),
            }
        }

        None => todo!(),
    };
}
