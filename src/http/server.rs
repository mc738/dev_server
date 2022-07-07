use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use regex::Regex;

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
    /// Start the http server.
    ///
    /// # Errors
    ///
    /// This function will return an error if TcpListener can not be bond to the address.
    pub fn start(
        address: String,
        log: &Log,
        sub_sender: Sender<Subscription>,
        base_path: String,
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
                                let bp = base_path.clone();
                                connection_pool
                                    .execute(|| handle_connection(stream, request_logger, ss, bp));
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
    /// Creates a new [`ConnectionPool`].
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
    /// Creates a new [`Worker`].
    ///
    /// # Panics
    ///
    /// Panics if a lock can ot be gained on the receiver or a job received.
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            job();
        });

        Worker { id, thread }
    }
}

/// Handle a connection from a client.
///
/// # Panics
///
/// Panics if an issue with the logger, a file can not be read or a failure to write to the stream.
fn handle_connection(
    mut stream: TcpStream,
    logger: Logger,
    sub_sender: Sender<Subscription>,
    base_path: String,
) {
    match HttpRequest::from_stream(&stream, &logger) {
        Ok(request) => match request.header.route.as_str() {
            "/ws/notify" => {
                logger
                    .log_info(format!("Update notification requested"))
                    .unwrap();
                handle_ws_connection(request, stream, sub_sender, logger);
            }
            route if route == "/" || route == "/index" || route == "/index.html" => {
                match File::open(format!("{}/index.html", base_path)) {
                    Ok(mut file) => {
                        let mut buf = Vec::new();

                        let mut doc = String::new();

                        file.read_to_string(&mut doc).unwrap();

                        file.read_to_end(&mut buf).unwrap();

                        let mut response = HttpResponse::create(
                            HttpStatus::Ok,
                            "text/html".to_string(),
                            HashMap::new(),
                            Some(inject_script(&doc).as_bytes().to_vec()),
                        );

                        stream.write(&response.to_bytes()).unwrap();
                    }
                    Err(_) => todo!(),
                }
            }
            _ => match File::open(get_path(format!(
                "{}{}",
                base_path,
                request.header.route.clone()
            ))) {
                Ok(mut file) => {
                    let mut buf = Vec::new();

                    file.read_to_end(&mut buf).unwrap();

                    let mut response = HttpResponse::create(
                        HttpStatus::Ok,
                        get_content_type(request.header.route.clone()),
                        HashMap::new(),
                        Some(buf),
                    );

                    stream.write(&&response.to_bytes()).unwrap();

                    logger
                        .log_info(format!("Request received. Route: {}", request.header.route))
                        .unwrap();
                }
                Err(_) => {
                    let mut response = HttpResponse::create(
                        HttpStatus::NotFound,
                        "text/plain".to_string(),
                        HashMap::new(),
                        Some(b"Not found".to_vec()),
                    );

                    stream.write(&response.to_bytes()).unwrap();
                }
            },
        },
        Err(_) => todo!(),
    };
}

/// Get a file path from a route.
fn get_path(route: String) -> String {
    route
}

/// Handle a WebSocket connection.
///
/// # Panics
///
/// Panics if a failure with the logger.
fn handle_ws_connection(
    request: HttpRequest,
    mut stream: TcpStream,
    sub_sender: Sender<Subscription>,
    logger: Logger,
) {
    logger.log_debug("WS connection".to_string()).unwrap();

    match request.header.headers.get("SEC-WEBSOCKET-KEY") {
        Some(key) => {
            logger.log_info(format!("Key: {}", key)).unwrap();
            let ws_handshake = ws::handle_handshake(key);
            logger
                .log_debug(format!("Handshake: {}", ws_handshake))
                .unwrap();

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
                        sub_sender.send(Subscription::new(tx.clone())).unwrap();

                        match rx.recv() {
                            Ok(notification) => {
                                let (data, len) = match notification {
                                    crate::messaging::Notification::FileCreated(_) => {
                                        (b"File created", 12)
                                    }
                                    crate::messaging::Notification::FileUpdated(_) => {
                                        (b"File updated", 12)
                                    }
                                    crate::messaging::Notification::FileRemoved(_) => {
                                        (b"File removed", 12)
                                    }
                                    crate::messaging::Notification::FileRenamed(_, _) => {
                                        (b"File renamed", 12)
                                    }
                                };

                                let result =
                                    stream.write(&ws::handle_write(&mut data.to_vec(), len));

                                match result {
                                    Ok(_) => {}
                                    Err(e) => {
                                        logger
                                            .log_error(format!(
                                                "Failed sending to client, Error {}",
                                                e
                                            ))
                                            .unwrap();
                                        break;
                                    }
                                };
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

/// Get the content type from a path based on it's file extension.
fn get_content_type(path: String) -> String {
    match path {
        _ if path.ends_with(".css") => "text/css".to_string(),
        _ if path.ends_with(".js") => "application/javascript".to_string(),
        _ if path.ends_with(".png") => "image/png".to_string(),
        _ if path.ends_with(".jpg") || path.ends_with(".jpeg") => "image/jpeg".to_string(),
        _ => "text/plain".to_string(),
    }
}

/// Inject the handler script into a html document.
///
/// # Panics
///
/// Panics if the regex can not be created.
fn inject_script(document: &String) -> String {
    let re = Regex::new("</body>").unwrap();

    let replace = "<script>var ws = new WebSocket('ws://127.0.0.1:8080/ws/notify'); ws.onopen = function(evt) { console.log('Connected'); };  ws.onmessage = function (evt) { location.reload();  };</script>\n</body>";

    re.replace(document, replace).to_string()
}
