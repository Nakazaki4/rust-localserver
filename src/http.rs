use crate::config::HttpMethod;
use std::collections::HashMap;

pub struct Request {
    pub method: HttpMethod,
    pub uri: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct Response {}

impl Request {
    pub fn default() -> Self {
        Self {
            method: HttpMethod::GET,
            uri: String::new(),
            path: String::new(),
            version: String::from("1.1"),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
}

pub enum ParseState {
    RequestLine,
    Headers,
    Body { remaining: usize }, // Content-Length mode
    ChunkSize,                 // chunked: reading "1a3f\r\n"
    ChunkData { remaining: usize },
    ChunkTrailer, // after the "0\r\n" chunk
    Complete,
    Error(u16),
}

pub struct RequestParser {
    pub state: ParseState,
    pub request: Request,
}

impl RequestParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::RequestLine,
            request: Request::default(),
        }
    }

    pub fn parse(&mut self, buf: &[u8], max_body: Option<usize>) {
        loop {
            // match self.state {
            //     ParseState::RequestLine => {
            //         println!(self.take_line(buf));
            //     }
            // }
        }
    }

    fn take_line(self, buf: &[u8]) -> String {
        let s = String::from_utf8(buf.to_vec()).unwrap();
        s.split_whitespace().collect()
    }
}
