use crate::config::ServerConfig;
use mio::event::Event;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::Read;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::{fmt, io};

pub const SERVER_TOKEN_MAX: usize = 100;

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

struct Connection {
    pub stream: TcpStream,
    pub listener_token: Token,
    pub last_activity: Instant,
    pub read_buffer: Vec<u8>,
    pub is_request_complete: bool,
}
impl Connection {
    fn new(stream: TcpStream, listener_token: Token) -> Self {
        Self {
            stream,
            listener_token,
            last_activity: Instant::now(),
            read_buffer: Vec::with_capacity(4096),
            is_request_complete: false,
        }
    }
}

impl Server {
    /// Takes ALL server configs — there is exactly one Server and one Poll
    /// for the whole program, no matter how many server blocks exist.
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

    pub fn run(&mut self) {
        let mut events = Events::with_capacity(2048);
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
        let connection_token = event.token();

        let conn = match self.connections.get_mut(&connection_token) {
            Some(c) => c,
            None => return,
        };

        if event.is_readable() {
            let mut buf = [0u8; 4096];
            loop {
                match conn.stream.read(&mut buf) {
                    Ok(0) => {
                        self.close_connection();
                        return;
                    }
                    Ok(n) => {
                        conn.read_buffer.extend_from_slice(&buf[..n]);
                        conn.last_activity = Instant::now();
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        eprintln!("read error on {connection_token:?}: {e}");
                        self.close_connection(token);
                        return;
                    }
                };
            }

            // self.parse_request(event);
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
                        Interest::READABLE,
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

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("listeners", &self.listeners.keys().collect::<Vec<_>>())
            .field("connections", &self.connections.len())
            .field("next_token", &self.next_token)
            .finish()
    }
}
