use std::{collections::HashMap, fs};

#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_body_size: usize,
    pub routes: Vec<Route>,
    pub error_pages: HashMap<u16, String>,
    pub default: bool,
}

#[derive(Clone)]
pub struct Route {
    path: String,
    methods: Vec<u16>,
    root: String,
    index: Option<String>,
    redirect: Option<String>,
    upload: bool,
    directory_listing: bool,
    cgi_extension: Option<String>,
    cgi_interpreter: Option<String>,
}
enum HttpMethod {
    GET,
    POST,
    DELETE,
}

impl HttpMethod {
    fn parse(method: &str) -> Result<Self, String> {
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
            port: 8080,
            max_body_size: 1024 * 1024,
            routes: Vec::new(),
            error_pages: HashMap::new(),
            default: true,
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
                    let mut methods: Vec<HttpMethod> = Vec::new();
                    for m in &parts[1..] {
                        methods.push(HttpMethod::parse(m)?);
                    }
                }
                "upload" => route.upload = true,
                "directory_listing" => match value {
                    "false" => route.directory_listing = false,
                    "true" => route.directory_listing = true,
                    _ => panic!("directory_listing should be either false true"),
                },
                "index" => route.index = Some(value.to_string()),
                "cgi_extension" => route.cgi_extension = Some(value.to_string()),
                "redirect" => route.redirect = Some(value.to_string()),
                _ => panic!("{}", format!("unsupported route config key: {}", key)),
            }
        } else if let Some(server) = current_server.as_mut() {
            let value = parts[1].to_string();
            match key {
                "host" => server.host = value,
                "port" => server.port = value.parse().unwrap(),
                "default" => server.default = value == "true",
                "max_body_size" => server.max_body_size = value.parse().unwrap(),
                "error_page" => {
                    if parts.len() != 3 {
                        return Err(format!("line {lineno}: error_page expects <code> <path>"));
                    }
                }
                _ => panic!("{}", format!("unsupported server config key: {}", key)),
            }
        } else {
            //error
        }
    }
    Ok(app_config)
}

// #[derive(Debug)]
// pub struct Route {
//     pub path: String,
//     pub methods: Vec<HttpMethod>, // was [HttpMethod; 3] — can't hold "GET" alone or "GET POST"
//     pub root: String,
//     pub index: Option<String>,
//     pub redirect: Option<String>,
//     pub upload: bool,
//     pub directory_listing: bool, // fixed the "diretory" typo
//     pub cgi_extension: Option<String>,
//     pub cgi_interpreter: Option<String>,
// }

// impl Default for Route {
//     fn default() -> Self {
//         Self {
//             path: String::new(),
//             methods: Vec::new(),
//             root: String::new(),
//             index: None,
//             redirect: None,
//             upload: false,
//             directory_listing: false,
//             cgi_extension: None,
//             cgi_interpreter: None,
//         }
//     }
// }
