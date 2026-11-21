use std::{
    fs::File,
    io::{BufReader, Read},
};

use crate::{
    config::{HttpMethod, Route, ServerConfig},
    http::Request,
};

pub struct Handler {}
impl Handler {
    pub fn request_handler(request: &Request, config: &ServerConfig) -> bool {
        if !Self::valid_request(request, config) {
            return false;
        }
        let route = Self::get_route(request, config);
        let file = route.unwrap().index;
        let dir = route.unwrap().root;
        match request.method {
            HttpMethod::GET => {
                // the user asking for something that is in uri in the config file
                let file = File::open(file + "/" + route).unwrap();
                let mut buf_reader = BufReader::new(file);
                let mut content = String::new();
                buf_reader.read_to_string(&mut content);
            }
            HttpMethod::POST => {}
            HttpMethod::DELETE => {}
        }

        return true;
    }

    fn valid_request(request: &Request, config: &ServerConfig) -> bool {
        let Some(route) = config.routes.iter().find(|r| r.path == request.path) else {
            return false;
        };
        route.methods.contains(&request.method)
    }

    fn get_route(request: &Request, config: &ServerConfig) -> Option<&Route> {
        config.routes.iter().find(|r| r.path == request.path)
    }
}
