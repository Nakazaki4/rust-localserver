use std::{collections::HashMap, fs};

pub struct AppConfig {
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

pub fn parse_config(path: &str) -> Result<AppConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config = AppConfig {
        servers: Vec::new(),
    };

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
            if let Some(route) = current_route {
                if let Some(server) = current_server {
                    server.routes.push(route);
                }
            } else if let Some(server) = current_server {
                app_config.push(current_server);
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
                        methods.push(HttpMethod.parse(m));
                    }
                }
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
            }
        } else {
            //error
        }
        if parts.len() == 3 {
            let status_code = parts[1];
            let html_path = parts[2];

            current_server
                .unwrap()
                .error_pages
                .insert(status_code.parse().unwrap(), html_path.to_string());
        } else if current_route.is_none() && parts.len() == 2 {
        } else if parts.len() == 2 {
        }
    }
    Ok(config)
}


// use std::{collections::HashMap, fs};

// #[derive(Debug)]
// pub struct AppConfig {
//     pub servers: Vec<ServerConfig>,
// }

// #[derive(Debug)]
// pub struct ServerConfig {
//     pub host: String,
//     pub port: u16,
//     pub max_body_size: usize,
//     pub routes: Vec<Route>,
//     pub error_pages: HashMap<u16, String>,
//     pub default: bool,
// }

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

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum HttpMethod {
//     GET,
//     POST,
//     DELETE,
// }

// impl HttpMethod {
//     fn parse(s: &str) -> Result<Self, String> {
//         match s {
//             "GET" => Ok(HttpMethod::GET),
//             "POST" => Ok(HttpMethod::POST),
//             "DELETE" => Ok(HttpMethod::DELETE),
//             other => Err(format!("unknown HTTP method: {other}")),
//         }
//     }
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

// impl Default for ServerConfig {
//     fn default() -> Self {
//         Self {
//             host: "127.0.0.1".to_string(),
//             port: 8080,
//             max_body_size: 1024 * 1024,
//             routes: Vec::new(),
//             error_pages: HashMap::new(),
//             default: false, // was true — see note below
//         }
//     }
// }

// pub fn parse_config(path: &str) -> Result<AppConfig, String> {
//     let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
//     let mut config = AppConfig { servers: Vec::new() };

//     let mut current_server: Option<ServerConfig> = None;
//     let mut current_route: Option<Route> = None;

//     for (i, raw_line) in content.lines().enumerate() {
//         let lineno = i + 1;
//         let line = raw_line.trim();
//         if line.is_empty() || line.starts_with('#') {
//             continue;
//         }

//         // --- server block open ---
//         if line == "server {" {
//             if current_server.is_some() {
//                 return Err(format!("line {lineno}: nested 'server' block"));
//             }
//             current_server = Some(ServerConfig::default());
//             continue;
//         }

//         // --- location block open ---
//         if line.starts_with("location") && line.ends_with('{') {
//             let parts: Vec<&str> = line.split_whitespace().collect();
//             let mut route = Route::default();
//             if parts.len() >= 2 {
//                 route.path = parts[1].to_string();
//             }
//             current_route = Some(route);
//             continue;
//         }

//         // --- closing brace: ends a location OR a server ---
//         if line == "}" {
//             if let Some(route) = current_route.take() {
//                 let server = current_server
//                     .as_mut()
//                     .ok_or_else(|| format!("line {lineno}: location outside of a server"))?;
//                 server.routes.push(route);
//             } else if let Some(server) = current_server.take() {
//                 config.servers.push(server);
//             } else {
//                 return Err(format!("line {lineno}: unexpected '}}'"));
//             }
//             continue;
//         }

//         // --- key/value directive (strip trailing ';') ---
//         let line = line.trim_end_matches(';').trim();
//         let parts: Vec<&str> = line.split_whitespace().collect();
//         if parts.is_empty() {
//             continue;
//         }
//         let key = parts[0];

//         if let Some(route) = current_route.as_mut() {
//             match key {
//                 "root" => route.root = value_of(&parts)?,
//                 "index" => route.index = Some(value_of(&parts)?),
//                 "redirect" => route.redirect = Some(value_of(&parts)?),
//                 "upload" => route.upload = value_of(&parts)? == "true",
//                 "directory_listing" => route.directory_listing = value_of(&parts)? == "true",
//                 "cgi_extension" => route.cgi_extension = Some(value_of(&parts)?),
//                 "cgi_interpreter" => route.cgi_interpreter = Some(value_of(&parts)?),
//                 "methods" => {
//                     let mut methods = Vec::new();
//                     for m in &parts[1..] {
//                         methods.push(HttpMethod::parse(m)?);
//                     }
//                     route.methods = methods;
//                 }
//                 other => return Err(format!("line {lineno}: unknown route key '{other}'")),
//             }
//         } else if let Some(server) = current_server.as_mut() {
//             match key {
//                 "host" => server.host = value_of(&parts)?,
//                 "port" => {
//                     server.port = value_of(&parts)?
//                         .parse()
//                         .map_err(|_| format!("line {lineno}: invalid port"))?
//                 }
//                 "default" => server.default = value_of(&parts)? == "true",
//                 "max_body_size" => {
//                     server.max_body_size = value_of(&parts)?
//                         .parse()
//                         .map_err(|_| format!("line {lineno}: invalid max_body_size"))?
//                 }
//                 "error_page" => {
//                     if parts.len() != 3 {
//                         return Err(format!("line {lineno}: error_page expects <code> <path>"));
//                     }
//                     let code: u16 = parts[1]
//                         .parse()
//                         .map_err(|_| format!("line {lineno}: invalid status code"))?;
//                     server.error_pages.insert(code, parts[2].to_string());
//                 }
//                 other => return Err(format!("line {lineno}: unknown server key '{other}'")),
//             }
//         } else {
//             return Err(format!("line {lineno}: directive outside any block"));
//         }
//     }

//     Ok(config)
// }

// fn value_of(parts: &[&str]) -> Result<String, String> {
//     parts
//         .get(1)
//         .map(|s| s.to_string())
//         .ok_or_else(|| format!("directive '{}' is missing a value", parts[0]))
// }