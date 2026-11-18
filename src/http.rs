use crate::config::HttpMethod;
use std::collections::HashMap;

pub struct Request {
    pub method: HttpMethod,
    pub uri: String,
    pub path: String,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub cookies: HashMap<String, String>,
}

pub struct Response {}

impl Request {
    pub fn default() -> Self {
        Self {
            method: HttpMethod::GET,
            uri: String::new(),
            path: String::new(),
            http_version: String::from("1.1"),
            headers: HashMap::new(),
            body: Vec::new(),
            cookies: HashMap::new(),
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
    pub pos: usize,
}

const MAX_HEADER_SIZE: usize = 1000 * 8;

impl RequestParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::RequestLine,
            request: Request::default(),
            pos: 0,
        }
    }

    pub fn parse(&mut self, buf: &[u8], max_body: Option<usize>) {
        loop {
            match self.state {
                ParseState::RequestLine => {
                    if self.pos == 0 && buf.len() > MAX_HEADER_SIZE {
                        self.state = ParseState::Error(400);
                    }
                    let Some(line) = self.take_line(buf) else {
                        return;
                    };
                    if line.is_empty() {
                        self.state = ParseState::Error(400);
                    }
                    if let Err(code) = self.parse_request_line(&line) {
                        self.state = ParseState::Error(code);
                        return;
                    };
                    self.state = ParseState::Headers;
                }
                ParseState::Headers => {
                    let Some(line) = self.take_line(buf) else {
                        return;
                    };

                    if line.is_empty() {
                        if self.request.http_version == "HTTP/1.1"
                            && self.request.header("host").is_none()
                        {
                            self.state = ParseState::Error(400);
                            return;
                        }
                        self.request.parse_cookies();
                        self.state = self.body_start_state();
                    } else if let Err(code) = self.parse_headers(&line) {
                        self.state = ParseState::Error(code);
                        return;
                    }
                    let content_length = self.request.headers.get("content-length")
                    .unwrap()
                    .parse()
                    .unwrap();
                    self.state = ParseState::Body { remaining: content_length }
                }
                ParseState::Body { remaining } => {
                    match self.request.is_chunked() {
                        true => , // request chunked means we need to keep reading
                        false => self.take_multilines(buf), // unchunked should be done in one read
                    }
                }
            }
        }
    }

    fn body_start_state(&mut self) -> ParseState {
        if self.request.is_chunked() {
            ParseState::ChunkSize
        } else {
            match self.request.content_length() {
                Some(0) => ParseState::Complete,
                Some(n)=> ParseState::Body { remaining:  }
                None =>,
            }
        }
    }

    fn take_body_chunks(&mut self, section: &[u8]) -> Result<(), u16> {
        let c = 
    }

    fn take_multilines(&mut self, section: &[u8]) -> Result<(), u16> {
            self.request.body.extend_from_slice(section);
        Ok(())
    }

    fn parse_headers(&mut self, section: &String) -> Result<(), u16> {
        let lines: Vec<&str> = section.lines().collect();
        
        for line in lines {
            if line.len() !=2 {
                return Err(400);
            }
            let mut parts  = line.split_ascii_whitespace();
            let key = parts.next().unwrap().to_string();
            let value = parts.next().unwrap().to_string();
            match key.to_ascii_lowercase().as_str() {
                "content-type" | "transfer-encoding" | "cookie" | "connection" | "content-length"  => {
                    self.request.headers.insert(key, value);
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_request_line(&mut self, line: &String) -> Result<(), u16> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(400);
        }
        let (m, uri, ver) = (parts[0], parts[1], parts[2]);
        if ver != "HTTP/1.1" && ver != "HTTP/1.0" {
            return Err(505);
        }

        self.request.method = HttpMethod::parse(m).map_err(|_| 405).unwrap();
        self.request.uri = uri.to_string();
        self.request.http_version = ver.to_string();

        let (path, _) = match uri.split_once('?') {
            Some((p, q)) => (p, Some(q.to_string())),
            None => (uri, None),
        };

        self.request.path = path.to_string();
        Ok(())
    }

    fn take_line(&mut self, buf: &[u8]) -> Option<String> {
        let slice = &buf[self.pos..];
        let end = slice.windows(2).position(|w| w == b"\r\n")?;
        let line = String::from_utf8_lossy(&slice[..end]).into_owned();
        self.pos += end + 2;
        Some(line)
    }
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn is_chunked(&self) -> bool {
    self.header("transfer-encoding")
        .map(|v| {
            v.split(',')
                .last()
                .map(|s| s.trim().eq_ignore_ascii_case("chunked"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

    pub fn host(&self) -> Option<&str> {
        self.header("host")
            .map(|h| h.split(':').next().unwrap_or(h))
    }

    pub fn parse_cookies(&mut self) {
        // cookie: name=ve; name=fe;
        if let Some(raw) = self.header("cookie").map(str::to_string) {
            for pair in raw.split(";") {
                if let Some((k, v)) = pair.split_once('=') {
                    self.cookies
                        .insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
    pub fn content_length(&self) -> Option<usize> {
        self.header("content-length")
            .and_then(|v| v.trim().parse().ok())
    }
}
