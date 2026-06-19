use std::collections::HashMap;

pub struct Config {
    servers: Vec<ServerConfig>,
}

pub struct ServerConfig {
    host: String,
    port: u16,
    max_body_size: usize,
    routes: Vec<Route>,
    error_pages: HashMap<u16, String>,
    default: bool,
}

pub struct Route {
    path: String,
    methods: [HttpMethod; 3],
    root: String,
    index: Option<String>,
    redirect: Option<String>,
    upload: bool,
    diretory_listing: bool,
    cgi_extension: Option<String>,
    cgi_interpreter: Option<String>,
}
enum HttpMethod {
    GET,
    POST,
    DELETE,
}
