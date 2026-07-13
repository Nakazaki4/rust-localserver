use std::{collections::HashMap, fs};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub ports: Vec<u16>,
    pub default: bool,
    pub max_body_size: usize, // 413 when exceeded
    pub error_pages: HashMap<u16, String>,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone)]
pub struct Route {
    path: String,
    methods: Vec<HttpMethod>,
    root: String,
    index: Option<String>,
    redirect: Option<String>,
    upload: bool,
    directory_listing: bool,
    cgi_extension: Option<String>,
    cgi_interpreter: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    DELETE,
}

impl HttpMethod {
    pub fn parse(method: &str) -> Result<Self, String> {
        match method {
            "GET" => return Ok(HttpMethod::GET),
            "POST" => return Ok(HttpMethod::POST),
            "DELETE" => return Ok(HttpMethod::DELETE),
            _ => panic!("wrong http method"),
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        Self {
            path: String::new(),
            methods: Vec::new(),
            root: String::new(),
            index: None,
            redirect: None,
            upload: false,
            directory_listing: false,
            cgi_extension: None,
            cgi_interpreter: None,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            ports: Vec::new(),
            default: false,
            max_body_size: 1024 * 1024,
            error_pages: HashMap::new(),
            routes: Vec::new(),
        }
    }
}

pub fn parse_config(path: &str) -> Result<Vec<ServerConfig>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    let mut app_config = Vec::new();
    let mut current_server: Option<ServerConfig> = None;
    let mut current_route: Option<Route> = None;

    for (i, raw_line) in content.lines().enumerate() {
        let lineno = i + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // check if server
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
            // this means we have to add the location to routes in to current_server
            if let Some(route) = current_route.take() {
                if let Some(server) = current_server.as_mut() {
                    server.routes.push(route);
                }
            } else if let Some(server) = current_server.take() {
                app_config.push(server);
            }
            continue;
        }

        let line = raw_line.trim_end_matches(";").trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let key = parts[0];

        if let Some(route) = current_route.as_mut() {
            let value = parts[1];
            match key {
                "root" => route.root = value.to_string(),
                "methods" => {
                    for m in &parts[1..] {
                        route.methods.push(HttpMethod::parse(m)?);
                    }
                }
                "upload" => {
                    route.upload = match value {
                        "true" => true,
                        "false" => false,
                        _ => panic!("upload type should be boolean"),
                    }
                }
                "directory_listing" => {
                    route.directory_listing = match value {
                        "false" => false,
                        "true" => true,
                        _ => panic!("directory_listing should be either false true"),
                    }
                }
                "index" => route.index = Some(value.to_string()),
                "cgi_extension" => route.cgi_extension = Some(value.to_string()),
                "cgi_interpreter" => route.cgi_interpreter = Some(value.to_string()),
                "redirect" => route.redirect = Some(value.to_string()),
                _ => panic!("{}", format!("unsupported route config key: {}", key)),
            }
        } else if let Some(server) = current_server.as_mut() {
            let value = parts[1].to_string();
            match key {
                "host" => server.host = value,
                "port" => {
                    for p in &parts[1..] {
                        let port: u16 = p
                            .parse()
                            .map_err(|_| format!("line {lineno}: invalid port `{p}`"))?;
                        if server.ports.contains(&port) {
                            return Err(format!(
                                "line {lineno}: port {port} listed twice in one server"
                            ));
                        }
                        server.ports.push(port);
                    }
                }
                "default" => server.default = value == "true",
                "max_body_size" => server.max_body_size = value.parse().unwrap(),
                "error_page" => {
                    if parts.len() != 3 {
                        return Err(format!("line {lineno}: error_page expects <code> <path>"));
                    }
                    server
                        .error_pages
                        .insert(parts[1].parse().unwrap(), parts[2].to_string());
                }
                _ => panic!("{}", format!("unsupported server config key: {}", key)),
            }
        } else {
            //error
        }
    }
    Ok(app_config)
}
