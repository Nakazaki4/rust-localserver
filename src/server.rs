use std::collections::HashMap;
use mio::Poll;
use crate::config::{Route, ServerConfig};

pub struct Server {
    routes: Vec<Route>,
    poll: Poll,
    config: ServerConfig,
    listeners: Box<HashMap<Token, ListenerEntry>>
}
