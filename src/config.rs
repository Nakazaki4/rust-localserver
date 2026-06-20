use std::{collections::HashMap, fs};

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

impl Default for Route {
    fn default() -> Self {
        Self {
            path: "".to_string(),
            methods: [HttpMethod::GET, HttpMethod::POST, HttpMethod::DELETE],
            root: "".to_string(),
            index: None,
            redirect: None,
            upload: false,
            diretory_listing: false,
            cgi_extension: None,
            cgi_interpreter: None,
        }
    }
}

// impl Route {
// fn new_route() -> Self {
//     Self {
//         path: "".to_string(),
//         methods: [HttpMethod::GET, HttpMethod::POST, HttpMethod::DELETE],
//         root: "".to_string(),
//         index: None,
//         redirect: None,
//         upload: false,
//         diretory_listing: false,
//         cgi_extension: None,
//         cgi_interpreter: None,
//     }
// }
// }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_body_size: 1024 * 1024,
            routes: Vec::new(),
            error_pages: HashMap::new(),
            default: true,
        }
    }
}

pub fn parse_config(path: &str) -> Result<Config, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config = Config {
        servers: Vec::new(),
    };

    let mut current_server: Option<ServerConfig> = None;
    let mut current_route: Option<Route> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // check if server
        //
        if line == "server {" {
            current_server = Some(ServerConfig::default());
            continue;
        }

        if line.starts_with("location") && line.ends_with("{") {
            // that means we are adding a route
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut route = Route::default();
            if parts.len() >= 2 {
                route.path = parts[1].to_string();
            }
            current_route = Some(route);
            continue;
        }
        if line == "}" {
            
        }
    }
    ok()
}
