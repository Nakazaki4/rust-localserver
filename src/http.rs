use crate::config::HttpMethod;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Request {
    pub method: HttpMethod,
    pub uri: String,
    pub path: String,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub cookies: HashMap<String, String>,
}

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

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug)]
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
                        eprintln!(
                            "[parse:RequestLine] buffer too large: {} bytes (max {}) -> 400",
                            buf.len(),
                            MAX_HEADER_SIZE
                        );
                        self.state = ParseState::Error(400);
                    }
                    let Some(line) = self.take_line(buf) else {
                        break;
                    };
                    if line.is_empty() {
                        eprintln!(
                            "[parse:RequestLine] empty request line at pos {} -> 400",
                            self.pos
                        );
                        self.state = ParseState::Error(400);
                        break;
                    }
                    if let Err(code) = self.parse_request_line(&line) {
                        eprintln!(
                            "[parse:RequestLine] invalid request line {:?} -> {}",
                            line, code
                        );
                        self.state = ParseState::Error(code);
                        break;
                    };
                    self.state = ParseState::Headers;
                }
                ParseState::Headers => {
                    let Some(line) = self.take_line(buf) else {
                        break;
                    };

                    if line.is_empty() {
                        if self.request.http_version == "HTTP/1.1"
                            && self.request.header("host").is_none()
                        {
                            eprintln!(
                                "[parse:Headers] HTTP/1.1 request missing Host header -> 400"
                            );
                            self.state = ParseState::Error(400);
                            break;
                        }
                        self.request.parse_cookies();
                        self.state = self.body_start_state();
                    } else if let Err(code) = self.parse_headers(&line) {
                        eprintln!("[parse:Headers] invalid header line {:?} -> {}", line, code);
                        self.state = ParseState::Error(code);
                        break;
                    }

                    if matches!(self.request.method, HttpMethod::GET) {
                        self.state = ParseState::Complete;
                        break;
                    }
                    self.state = ParseState::Body { remaining: 1 };
                }
                ParseState::Body { remaining } => {
                    let _ = match self.request.is_chunked() {
                        true => self.take_body_chunk(buf, &max_body.unwrap()), // request chunked means we need to keep reading
                        false => self.take_body(buf), // unchunked should be done in one read
                    };
                }

                _ => {}
            }
        }
    }

    fn body_start_state(&mut self) -> ParseState {
        if self.request.is_chunked() {
            ParseState::ChunkSize
        } else {
            match self.request.content_length() {
                Some(0) => ParseState::Complete,
                Some(n) => ParseState::Body { remaining: n },
                None => ParseState::RequestLine,
            }
        }
    }

    fn find(&mut self, chunk: &[u8], e: &[u8]) -> Option<usize> {
        chunk.windows(e.len()).position(|w| w == e)
    }

    fn take_body_chunk(&mut self, section: &[u8], max_body: &usize) -> Result<(), u16> {
        let mut body = Vec::new();
        let mut pos = 0;

        loop {
            // size line
            let Some(rel) = self.find(&section[pos..], b"\r\n") else {
                return Err(400);
            };
            let size_line = &section[pos..pos + rel];
            pos += rel + 2;

            let size_part = size_line.split(|&b| b == b';').next().unwrap_or(&[]);
            let size_str = std::str::from_utf8(size_part).map_err(|_| 400u16)?.trim();
            let size = usize::from_str_radix(size_str, 16).map_err(|_| 400u16)?;

            if size == 0 {
                loop {
                    let Some(rel) = self.find(&section[pos..], b"\r\n") else {
                        return Err(400);
                    };
                    if rel == 0 {
                        self.request.body = body;
                        self.state = ParseState::Complete;
                        return Ok(());
                    }
                    pos += rel + 2;
                }
            }

            if body.len() + size > *max_body {
                return Err(413);
            }
            if section.len() < pos + size + 2 {
                return Ok(());
            }
            body.extend_from_slice(&section[pos..pos + size]);
            if &section[pos + size..pos + size + 2] != b"\r\n" {
                return Err(400);
            }
            pos += size + 2;
            self.pos += size;
        }
    }

    fn take_body(&mut self, section: &[u8]) -> Result<(), u16> {
        self.request.body.extend_from_slice(section);
        self.pos += self.request.body.len();
        Ok(())
    }

    fn parse_headers(&mut self, section: &String) -> Result<(), u16> {
        let lines: Vec<&str> = section.lines().collect();

        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                return Err(400);
            }
            let mut parts = line.split_ascii_whitespace();
            let key = parts
                .next()
                .unwrap()
                .to_string()
                .trim_end_matches(':')
                .to_ascii_lowercase();
            let value = parts.next().unwrap().to_string();
            self.request.headers.insert(key.to_ascii_lowercase(), value);
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
