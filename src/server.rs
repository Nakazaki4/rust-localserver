use crate::config::ServerConfig;
use crate::handler::request_handler;
use crate::http::{ParseState, RequestParser};
use mio::event::Event;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::{fmt, io};

pub const SERVER_TOKEN_MAX: usize = 4;

pub struct Server {
    poll: Poll,
    listeners: HashMap<Token, ListenerEntry>,
    connections: HashMap<Token, Connection>,
    pending_cgi: HashMap<Token, PendingCgi>,
    sessions: HashMap<String, Session>,
    next_token: usize,
}

pub struct Session {
    pub data: HashMap<String, String>,
    pub last_seen: Instant,
}

struct ListenerEntry {
    listener: TcpListener,
    addr: SocketAddr,
    configs: Vec<ServerConfig>,
}

struct PendingCgi {
    started_at: Instant,
    output: Vec<u8>,
    io_token: Token,
    child_process: std::process::Child,
    stdout: std::process::ChildStdout,
}

#[derive(Debug)]
struct Connection {
    pub stream: TcpStream,
    pub listener_token: Token,
    pub last_activity: Instant,
    pub read_buffer: Vec<u8>,
    pub parser: RequestParser,
    pub resolved_max_body: Option<usize>,
}
impl Connection {
    fn new(stream: TcpStream, listener_token: Token) -> Self {
        Self {
            stream,
            listener_token,
            last_activity: Instant::now(),
            read_buffer: Vec::with_capacity(4096),
            parser: RequestParser::new(),
            resolved_max_body: None,
        }
    }
}

impl Server {
    pub fn new() -> Result<Self, String> {
        Ok(Server {
            poll: Poll::new().map_err(|e| format!("failed to create poll: {e}"))?,
            listeners: HashMap::new(),
            connections: HashMap::new(),
            pending_cgi: HashMap::new(),
            sessions: HashMap::new(),
            next_token: SERVER_TOKEN_MAX,
        })
    }

    // binding the listner to epoll instance
    pub fn bind(configs: Vec<ServerConfig>) -> Result<Server, String> {
        let mut server = Self::new()?;

        let mut by_addr: Vec<(SocketAddr, Vec<ServerConfig>)> = Vec::new();
        for sc in configs {
            for port in &sc.ports {
                let addr: SocketAddr = format!("{}:{}", sc.host, port)
                    .parse()
                    .map_err(|e| format!("invalid address {}:{}: {e}", sc.host, port))?;

                match by_addr.iter_mut().find(|(a, _)| *a == addr) {
                    Some((_, group)) => group.push(sc.clone()),
                    None => by_addr.push((addr, vec![sc.clone()])),
                }
            }
        }

        let mut token_counter = 0;
        for (addr, group) in by_addr {
            match TcpListener::bind(addr) {
                Ok(mut listener) => {
                    let token = Token(token_counter);
                    token_counter += 1;
                    if token_counter >= SERVER_TOKEN_MAX {
                        return Err("too many listeners".to_string());
                    }

                    server
                        .poll
                        .registry()
                        .register(&mut listener, token, Interest::READABLE)
                        .map_err(|e| format!("register {addr}: {e}"))?;

                    server.listeners.insert(
                        token,
                        ListenerEntry {
                            listener,
                            addr,
                            configs: group,
                        },
                    );
                }
                Err(e) => eprintln!("warning: failed to bind {addr}: {e}"),
            }
        }

        if server.listeners.is_empty() {
            return Err("no listener could be bound".to_string());
        }
        Ok(server)
    }

    // start listening for connections
    pub fn run(&mut self) {
        let mut events = Events::with_capacity(1024);
        loop {
            if let Err(e) = self
                .poll
                .poll(&mut events, Some(Duration::from_millis(2000)))
            {
                eprintln!("Poll error :{e}");
            };
            for event in &events {
                let token = event.token();

                if self.listeners.contains_key(&token) {
                    // if the token belongs to a listener then should accept the connection
                    self.accept_connection(token);
                } else if self.connections.contains_key(&token) {
                    // if the token belongs to a connection that means we have an event form epoll
                    self.handle_client_event(event);
                }
            }
        }
    }

    fn handle_client_event(&mut self, event: &Event) {
        let token = event.token();

        // 1. read: drain the socket into conn.read_buffer
        match self.read_from_client(token) {
            Ok(true) => {}
            Ok(false) => {
                self.close_connection(token);
                return;
            }
            Err(e) => {
                eprintln!("read error on {token:?}: {e}");
                self.close_connection(token);
                return;
            }
        }

        // 2. process: parse what we have; None means the request is incomplete
        let Some(response) = self.process_request(token) else {
            return;
        };
        // println!("{:?}", String::from_utf8(response));

        // 3. write
        if let Err(e) = self.write_response(token, &response) {
            eprintln!("write error on {token:?}: {e}");
        }
        self.close_connection(token);
    }

    /// Reads everything available. Ok(true) = connection still open, Ok(false) = peer closed.
    fn read_from_client(&mut self, token: Token) -> io::Result<bool> {
        let Some(conn) = self.connections.get_mut(&token) else {
            return Ok(false);
        };

        let mut buf = [0u8; 4096];
        loop {
            match conn.stream.read(&mut buf) {
                Ok(0) => return Ok(false),
                Ok(n) => {
                    conn.read_buffer.extend_from_slice(&buf[..n]);
                    conn.last_activity = Instant::now();
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        // println!(
        //     "---- request on {token:?} ({} bytes) ----\n{}\n---- end ----",
        //     conn.read_buffer.len(),
        //     String::from_utf8_lossy(&conn.read_buffer)
        // );
        Ok(true)
    }

    /// Feeds the buffered bytes to the parser. Returns the response to send,
    /// or None if more data is needed.
    fn process_request(&mut self, token: Token) -> Option<Vec<u8>> {
        let conn = self.connections.get_mut(&token)?;
        conn.parser.parse(&conn.read_buffer, conn.resolved_max_body);
        match conn.parser.state {
            ParseState::Error(code) => Some(simple_response(code)),
            ParseState::Complete => {
                let entry = self.listeners.get(&conn.listener_token)?;
                let config = select_config(&entry.configs, conn.parser.request.header("host"));
                Some(request_handler(&conn.parser.request, config))
                // Some(simple_response(200))
            }
            _ => None,
        }
    }

    fn write_response(&mut self, token: Token, response: &[u8]) -> io::Result<()> {
        let conn = self
            .connections
            .get_mut(&token)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "connection gone"))?;
        conn.stream.write_all(response)
    }

    fn close_connection(&mut self, token: Token) {
        if let Some(mut conn) = self.connections.remove(&token) {
            let _ = self.poll.registry().deregister(&mut conn.stream);
        };
        if let Some(mut cgi) = self.pending_cgi.remove(&token) {
            let _ = cgi.child_process.kill();
            let _ = cgi.child_process.wait();
        }
    }

    fn accept_connection(&mut self, listener_token: Token) {
        loop {
            let accept_result = match self.listeners.get(&listener_token) {
                Some(entry) => entry.listener.accept(),
                None => return,
            };

            match accept_result {
                Ok((mut stream, _addr)) => {
                    let connection_token = Token(self.next_token);
                    self.next_token += 1;

                    if let Err(e) = self.poll.registry().register(
                        &mut stream,
                        connection_token,
                        Interest::READABLE | Interest::WRITABLE,
                    ) {
                        eprintln!("registring the connection to Poll failed {e}");
                        continue;
                    }

                    self.connections
                        .insert(connection_token, Connection::new(stream, listener_token));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("accept error on {listener_token:?}: {e}");
                    break;
                }
            }
        }
    }
}

/// Picks the server block whose `host` matches the Host header ("site-b.local:8080" -> "site-b.local"),
/// falling back to the first one (the default server for this address).
fn select_config<'a>(configs: &'a [ServerConfig], host_header: Option<&str>) -> &'a ServerConfig {
    let host_name = host_header.and_then(|h| h.split(':').next()).unwrap_or("");
    configs
        .iter()
        .find(|c| c.host.eq_ignore_ascii_case(host_name))
        .unwrap_or(&configs[0])
}

fn simple_response(code: u16) -> Vec<u8> {
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        505 => "HTTP Version Not Supported",
        _ => "Internal Server Error",
    };
    format!("HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .into_bytes()
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("listeners", &self.listeners.keys().collect::<Vec<_>>())
            .field("connections", &self.connections.len())
            .field("next_token", &self.next_token)
            .finish()
    }
}
