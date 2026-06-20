use crate::config::{Route, ServerConfig};
use mio::{Poll, Token};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

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
