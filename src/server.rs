use crate::config::{Route, ServerConfig};
use mio::net::TcpListener;
use mio::{Interest, Poll, Token};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::time::Instant;

const SERVER_TOKEN_MAX: usize = 100;

pub struct Server {
    routes: Vec<Route>,
    poll: Poll,
    config: ServerConfig,
    listeners: Box<HashMap<Token, ListenerEntry>>,
    connections: Box<HashMap<Token, Connection>>,
    pending_cgi: Box<HashMap<Token, PendingCgi>>,
    next_token: usize,
}

struct ListenerEntry {
    listener: TcpListener,
    server_idx: usize,
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
    pub server_idx: usize,
    pub last_activity: Instant,
    pub read_buffer: Vec<u8>,
    pub is_request_complete: bool,
}

impl Server {
    fn new(config: ServerConfig) -> Self {
        Server {
            routes: Vec::new(),
            poll: Poll::new().expect("failed to create mio poll"),
            config: config,
            listeners: Box::new(HashMap::new()),
            connections: Box::new(HashMap::new()),
            pending_cgi: Box::new(HashMap::new()),
            next_token: SERVER_TOKEN_MAX,
        }
    }

    pub fn bind(sc: ServerConfig, idx: usize) -> Result<(), String> {
        let mut server = Self::new(sc);

        let addr = format!("{}:{}", server.config.host, server.config.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        match TcpListener::bind(addr) {
            Ok(mut listener) => {
                let token = Token(idx);

                server
                    .poll
                    .registry()
                    .register(&mut listener, token, Interest::READABLE)
                    .map_err(|e| e.to_string())?;

                server.listeners.insert(
                    token,
                    ListenerEntry {
                        listener,
                        server_idx: idx,
                    },
                );
            }
            Err(e) => eprintln!("failed to bind {}: {}", addr, e),
        }
        if server.listeners.is_empty() {
            return Err("no ports were found".to_string());
        }
        Ok(())
    }
}
