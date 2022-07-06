use crate::logging::logger::Logger;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;

#[derive(Clone, Copy)]
pub enum HttpVerb {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}

pub enum HttpStatus {
    SwitchingProtocols,
    Ok,
    BadRequest,
    Unauthorized,
    NotFound,
    MethodNotAllowed,
    InternalError,
}

pub struct HttpRequest {
    pub header: HttpRequestHeader,
    pub body: Option<Vec<u8>>,
}

pub struct HttpRequestHeader {
    pub route: String,
    pub verb: HttpVerb,
    pub content_length: usize,
    pub headers: HashMap<String, String>,
    pub http_version: String,
}

pub struct HttpResponse {
    pub header: HttpResponseHeader,
    pub body: Option<Vec<u8>>,
}

pub struct HttpResponseHeader {
    pub http_version: String,
    pub status: HttpStatus,
    pub content_length: usize,
    //pub content_type: String,
    pub headers: HashMap<String, String>,
}

impl HttpVerb {
    /// Create a HttpVerb from a name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the name is unknown.
    pub fn from_str(data: &str) -> Result<HttpVerb, &'static str> {
        match data.to_uppercase().as_str() {
            "GET" => Ok(HttpVerb::GET),
            "HEAD" => Ok(HttpVerb::HEAD),
            "POST" => Ok(HttpVerb::POST),
            "PUT" => Ok(HttpVerb::PUT),
            "DELETE" => Ok(HttpVerb::DELETE),
            "CONNECT" => Ok(HttpVerb::CONNECT),
            "PATCH" => Ok(HttpVerb::PATCH),
            "OPTIONS" => Ok(HttpVerb::OPTIONS),
            "TRACE" => Ok(HttpVerb::TRACE),
            _ => Err("Unknown http verb"),
        }
    }

    /// Returns a reference to the name of this [`HttpVerb`].
    pub fn get_str(&self) -> &'static str {
        match self {
            HttpVerb::GET => "GET",
            HttpVerb::HEAD => "HEAD",
            HttpVerb::POST => "POST",
            HttpVerb::PUT => "PUT",
            HttpVerb::DELETE => "DELETE",
            HttpVerb::CONNECT => "CONNECT",
            HttpVerb::OPTIONS => "OPTIONS",
            HttpVerb::TRACE => "TRACE",
            HttpVerb::PATCH => "PATCH",
        }
    }
}

impl HttpStatus {
    /// Create a HttpStatus from a status code.
    ///
    /// # Errors
    ///
    /// This function will return an error if the status code is unknown.
    pub fn from_code(code: i16) -> Result<HttpStatus, &'static str> {
        match code {
            101 => Ok(HttpStatus::SwitchingProtocols),
            200 => Ok(HttpStatus::Ok),
            400 => Ok(HttpStatus::BadRequest),
            401 => Ok(HttpStatus::Unauthorized),
            404 => Ok(HttpStatus::NotFound),
            405 => Ok(HttpStatus::MethodNotAllowed),
            500 => Ok(HttpStatus::InternalError),
            _ => Err("Unknown response type code"),
        }
    }

    /// Returns the status code of this [`HttpStatus`].
    pub fn get_code(&self) -> i16 {
        match self {
            HttpStatus::SwitchingProtocols => 101,
            HttpStatus::Ok => 200,
            HttpStatus::BadRequest => 400,
            HttpStatus::Unauthorized => 401,
            HttpStatus::NotFound => 404,
            HttpStatus::MethodNotAllowed => 405,
            HttpStatus::InternalError => 500,
        }
    }

    /// Returns a reference to the name of this [`HttpStatus`].
    pub fn get_str(&self) -> &'static str {
        match self {
            HttpStatus::SwitchingProtocols => "Switching Protocols",
            HttpStatus::Ok => "OK",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
            HttpStatus::InternalError => "Internal Error",
        }
    }
}

impl HttpRequest {
    /// Create a new HttpRequest.
    pub fn create(
        route: String,
        verb: HttpVerb,
        content_type: String,
        addition_headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> HttpRequest {
        let len = match &body {
            None => 0,
            Some(b) => b.len(),
        };

        HttpRequest {
            header: HttpRequestHeader::create(route, verb, content_type, addition_headers, len),
            body,
        }
    }

    /// Create a HttpRequest from a TcpStream.
    ///
    /// # Panics
    ///
    /// Panics if the stream can not be read or there is an issue with the logger.
    ///
    /// # Errors
    ///
    /// Does not currently error, however it should error instead of panic.
    pub fn from_stream(
        mut stream: &TcpStream,
        logger: &Logger,
    ) -> Result<HttpRequest, &'static str> {
        let mut buffer = [0; 4096];
        let mut body: Vec<u8> = Vec::new();
        logger
            .log_debug(format!("Parsing http request header."))
            .unwrap();
        stream.read(&mut buffer).unwrap();
        logger.log_debug(format!("Read to buffer.")).unwrap();
        let (header, body_start_index) = HttpRequestHeader::create_from_buffer(buffer)?;
        let body = match (
            header.content_length > 0,
            body_start_index + header.content_length as usize > 4096,
        ) {
            // Short cut -> content length is 0 so no body
            (false, _) => None,
            // If the body_start_index + content length
            // the request of the body is bigger than the buffer and more reads needed
            (true, true) => {
                // TODO handle!
                None
            }
            // If the body_start_index + content length < 2048,
            // the body is in the initial buffer and no more reading is needed.
            (true, false) => {
                let end = body_start_index + header.content_length as usize;

                let body = buffer[body_start_index..end].to_vec();

                Some(body)
            }
        };

        Ok(HttpRequest { header, body })
    }

    /// Get the bytes of this [`HttpRequest`].
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Get the bytes for the header and append the response body.
        let mut bytes = self.header.to_bytes();

        if let Some(b) = &self.body {
            let mut body = b.clone();

            bytes.append(&mut body);
        }

        bytes
    }
}

impl HttpRequestHeader {
    /// Create a new HttpRequestHeader.
    pub fn create(
        route: String,
        verb: HttpVerb,
        content_type: String,
        addition_headers: HashMap<String, String>,
        content_length: usize,
    ) -> HttpRequestHeader {
        let http_version = String::from("HTTP/1.1");

        // Map the headers.
        let mut headers: HashMap<String, String> = HashMap::new();

        // Add any standardized headers.
        headers.insert("Server".to_string(), "Psionic 0.0.1".to_string());
        headers.insert("Content-Length".to_string(), format!("{}", content_length));
        headers.insert("Connection".to_string(), "Closed".to_string());
        headers.insert("Content-Type".to_string(), content_type);

        for (k, v) in addition_headers {
            headers.insert(k, v);
        }

        HttpRequestHeader {
            route,
            verb,
            content_length,
            headers,
            http_version,
        }
    }

    /// Create a new HttpRequestHeader from a buffer.
    ///
    /// # Errors
    ///
    /// This function will return an error if the request header is larger than the buffer.
    pub fn create_from_buffer(
        buffer: [u8; 4096],
    ) -> Result<(HttpRequestHeader, usize), &'static str> {
        for i in 0..buffer.len() {
            if i > 4
                && buffer[i] == 10
                && buffer[i - 1] == 13
                && buffer[i - 2] == 10
                && buffer[i - 3] == 13
            {
                // \r\n\r\n found, after this its the body.
                let header = String::from_utf8_lossy(&buffer[0..i]).into_owned();

                //println!("{}", header);

                let request = HttpRequestHeader::parse_from_string(header)?;

                return Ok((request, i + 1));
            }
        }

        Err("Request header larger than buffer")
    }

    /// Parse a HttpRequestHeader from a string.
    ///
    /// # Errors
    ///
    /// This function will not return an error if the HttpVerb can not be created.
    pub fn parse_from_string(data: String) -> Result<HttpRequestHeader, &'static str> {
        let split_header: Vec<&str> = data.split("\r\n").collect();

        let mut headers = HashMap::new();

        let mut content_length: usize = 0;

        let split_status_line: Vec<&str> = split_header[0].split(" ").collect();

        let verb = HttpVerb::from_str(split_status_line[0])?;
        let route = String::from(split_status_line[1]);
        let http_version = String::from(split_status_line[2]);

        for i in 1..split_header.len() {
            let split_item: Vec<&str> = split_header[i].split(": ").collect();

            // If the split item has more than 1 item, add a header.
            if split_item.len() > 1 {
                let k = String::from(split_item[0]).to_uppercase();
                let v = String::from(split_item[1]);

                // If the header item is `Content-Length` set it as such.
                if k == "CONTENT-LENGTH" {
                    match v.parse::<usize>() {
                        Ok(i) => content_length = i,
                        Err(_) => {}
                    }
                }

                headers.insert(k, v);
            }
        }

        Ok(HttpRequestHeader {
            route,
            verb,
            content_length,
            headers,
            http_version,
        })
    }

    /// Returns the string of this [`HttpRequestHeader`].
    pub fn get_string(&self) -> String {
        let mut header_string = String::new();

        header_string.push_str(&self.verb.get_str());
        header_string.push(' ');
        header_string.push_str(&self.route);
        header_string.push(' ');
        header_string.push_str(&self.http_version);

        header_string.push_str("\r\n");

        for header in &self.headers {
            header_string.push_str(&header.0);
            header_string.push_str(": ");
            header_string.push_str(&header.1);
            header_string.push_str("\r\n");
        }

        header_string.push_str("\r\n");
        header_string
    }

    /// Get the bytes of this [`HttpRequestHeader`].
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::from(self.get_string().as_bytes());

        bytes
    }
}

impl HttpResponse {
    /// Create a new HttpResponse.
    pub fn create(
        status: HttpStatus,
        content_type: String,
        addition_headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> HttpResponse {
        let len = match &body {
            None => 0,
            Some(b) => b.len(),
        };

        HttpResponse {
            header: HttpResponseHeader::create(status, content_type, addition_headers, len),
            body,
        }
    }

    /// Create a new HttpResponse from a TcpStream.
    ///
    /// # Panics
    ///
    /// Panics if the stream can not be read.
    ///
    /// # Errors
    ///
    /// This function will return an error if the HttpResponseHeader can not be created.
    pub fn from_stream(
        mut stream: &TcpStream, /*, logger: &Logger*/
    ) -> Result<HttpResponse, &'static str> {
        let mut buffer = [0; 4096];
        let mut body: Vec<u8> = Vec::new();
        //logger.log_debug( format!("Parsing http response header.")).unwrap();
        let read = stream.read(&mut buffer).unwrap();
        //logger.log_debug(format!("Read to buffer.")).unwrap();
        let (header, body_start_index) = HttpResponseHeader::create_from_buffer(buffer)?;
        let body = match (
            header.content_length > 0,
            body_start_index + header.content_length as usize > 4096,
        ) {
            // Short cut -> content length is 0 so no body
            (false, _) => None,
            // If the body_start_index + content length
            // the request of the body is bigger than the buffer and more reads needed
            (true, true) => {
                // TODO handle!
                None
            }
            // If the body_start_index + content length < 2048,
            // the body is in the initial buffer and no more reading is needed.
            (true, false) => {
                if read == body_start_index {
                    // Only head was send (might be general.
                    // Therefore clear the array
                    buffer.fill(0);
                    stream.read(&mut buffer).unwrap();
                    body = buffer[0..header.content_length].to_vec();
                } else {
                    let end = body_start_index + header.content_length as usize;
                    body = buffer[body_start_index..end].to_vec();
                }

                Some(body)
            }
        };

        Ok(HttpResponse { header, body })
    }

    /// Returns the bytes of this [`HttpResponse`].
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Get the bytes for the header and append the response body.
        let mut bytes = self.header.to_bytes();

        if let Some(b) = &self.body {
            let mut body = b.clone();

            bytes.append(&mut body);
        }

        bytes
    }
}

impl HttpResponseHeader {
    /// Create a new HttpResponseHeader.
    pub fn create(
        status: HttpStatus,
        content_type: String,
        addition_headers: HashMap<String, String>,
        content_length: usize,
    ) -> HttpResponseHeader {
        let http_version = String::from("HTTP/1.1");

        // Map the headers.
        let mut headers: HashMap<String, String> = HashMap::new();

        // Add any standardized headers.
        headers.insert("Server".to_string(), "Psionic 0.0.1".to_string());
        headers.insert("Content-Length".to_string(), format!("{}", content_length));
        headers.insert("Connection".to_string(), "Closed".to_string());
        headers.insert("Content-Type".to_string(), content_type);

        for (k, v) in addition_headers {
            headers.insert(k, v);
        }

        HttpResponseHeader {
            http_version,
            status,
            content_length,
            headers,
        }
    }

    /// Create a new HttpResponseHeader from a bufffer.
    ///
    /// # Errors
    ///
    /// This function will return an error if the header is bigger than the buffer.
    pub fn create_from_buffer(
        buffer: [u8; 4096],
    ) -> Result<(HttpResponseHeader, usize), &'static str> {
        for i in 0..buffer.len() {
            if i > 4
                && buffer[i] == 10
                && buffer[i - 1] == 13
                && buffer[i - 2] == 10
                && buffer[i - 3] == 13
            {
                // \r\n\r\n found, after this its the body.
                let header = String::from_utf8_lossy(&buffer[0..i]).into_owned();

                //println!("{}", header);

                let response = HttpResponseHeader::parse_from_string(header)?;

                return Ok((response, i + 1));
            }
        }

        Err("Request header larger than buffer")
    }

    /// Parse a HttpResponseHeader from a string.
    ///
    /// # Errors
    ///
    /// This function will return an error if HttpStatus can not be created.
    pub fn parse_from_string(data: String) -> Result<HttpResponseHeader, &'static str> {
        let split_header: Vec<&str> = data.split("\r\n").collect();

        let mut headers = HashMap::new();

        let mut content_length: usize = 0;

        let split_status_line: Vec<&str> = split_header[0].split(" ").collect();

        //let verb = HttpVerb::from_str(split_status_line[0])?;
        //let route = String::from(split_status_line[1]);
        let http_version = String::from(split_status_line[0]);
        //let response = split_status_line[1].parse::<i32>();

        let status = match split_status_line[1].parse::<i16>() {
            Ok(status_code) => HttpStatus::from_code(status_code),
            Err(_) => Err("Failed to parse status code"),
        }?;

        for i in 1..split_header.len() {
            //println!("Head: {}", split_header[i]);

            let split_item: Vec<&str> = split_header[i].split(": ").collect();

            // If the split item has more than 1 item, add a header.
            if split_item.len() > 1 {
                let k = String::from(split_item[0]).to_uppercase();
                let v = String::from(split_item[1]);

                // If the header item is `Content-Length` set it as such.
                if k == "CONTENT-LENGTH" {
                    match v.parse::<usize>() {
                        Ok(i) => content_length = i,
                        Err(_) => {}
                    }
                }

                headers.insert(k, v);
            }
        }

        Ok(HttpResponseHeader {
            headers,
            http_version,
            status,
            content_length,
        })
    }

    /// Returns the string of this [`HttpResponseHeader`].
    pub fn get_string(&self) -> String {
        // Create the header.
        let mut header_string = String::new();

        header_string.push_str(&self.http_version);
        header_string.push(' ');
        header_string.push_str(&self.status.get_code().to_string());
        header_string.push(' ');
        header_string.push_str(self.status.get_str());

        header_string.push_str("\r\n");

        for header in &self.headers {
            header_string.push_str(&header.0);
            header_string.push_str(": ");
            header_string.push_str(&header.1);
            header_string.push_str("\r\n");
        }

        header_string.push_str("\r\n");
        header_string
    }

    /// Returns the bytes of this [`HttpResponseHeader`].
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Get the bytes for the header and append the response body.
        let bytes = Vec::from(self.get_string());

        bytes
    }
}
