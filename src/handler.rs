use std::{os, path::{Path, PathBuf}};

use crate::{
    config::{HttpMethod, Route, ServerConfig},
    http::Request,
};

pub fn request_handler(request: &Request, config: &ServerConfig) -> Vec<u8> {
    // 404 if no route matches, 405 if the method isn't allowed
    let route = match resolve_route(request, config) {
        Ok(r) => r,
        Err(code) => return error_response(code),
    };

    // Redirect routes answer before anything else
    if let Some(target) = &route.redirect {
        return redirect_response(target);
    }
    match request.method {
        HttpMethod::GET => handle_get(request, route),
        HttpMethod::POST => handle_post(request, route),
        HttpMethod::DELETE => handle_delete(request, route),
    }
}

fn handle_post(request: &Request, route: &Route) -> Vec<u8> {
    // get the request body and store it in a file in the root
    let file_name = request
        .headers
        .get("content-disposition")
        .unwrap()
        .split(';')
        .map(|p| p.trim())
        .find_map(|p| p.strip_prefix("filename="))
        .map(|f| f.trim_matches('"'));
    let save_dir = PathBuf::from(&route.root);
    let file_path = save_dir.join(file_name.unwrap());
    match std::fs::write(&file_path, &request.body) {
        Ok(()) => {
            let body = format!(
                "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Upload successful</title></head>\
             <body><h1>Upload successful</h1></body></html>"
            );
            build_response(201, "text/html; charset=utf-8", body.into_bytes());
        }
        Err(e) => {
            eprintln!("failed to write {}: {e}", file_path.display());
            error_response(500);
        }
    }

    return Vec::new();
}
fn handle_delete(request: &Request, route: &Route) -> Vec<u8> {
    return Vec::new();
}

fn handle_get(_request: &Request, route: &Route) -> Vec<u8> {
    let index = route.index.as_deref().unwrap_or("");
    let dir = PathBuf::from(&route.root);
    if route.directory_listing {
        // then we list the root tree
        return directory_listing(&dir, &route.root);
    }

    let file_path = dir.join(index);
    let content_type = content_type(&file_path);

    match std::fs::read(&file_path) {
        Ok(body) => build_response(200, content_type, body),
        Err(_) => error_response(404),
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn directory_listing(dir: &PathBuf, url_path: &String) -> Vec<u8> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return error_response(500);
    };

    // Collect (name, is_dir), skipping what can't be read
    let mut items: Vec<(String, bool)> = entries
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (name, is_dir)
        })
        .collect();

    // folders first then files
    items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let base = format!("{}/", url_path.trim_end_matches('/'));
    let title = url_path;
    let mut html = format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Index of {title}</title></head>\
         <body><h1>Index of {title}</h1><hr><ul>"
    );

    if base != "/" {
        html.push_str("<li><a href=\"../\">../</a></li>");
    }

    for (name, is_dir) in items {
        let slash = if is_dir { "/" } else { "" };
        let href = format!("{base}{name}{slash}");
        html.push_str(&format!("<li><a href=\"{href}\">{name}{slash}</li>"));
    }

    build_response(200, "text/html; charset=utf-8", html.into_bytes())
}

fn resolve_route<'a>(request: &Request, config: &'a ServerConfig) -> Result<&'a Route, u16> {
    let Some(route) = config.routes.iter().find(|r| r.path == request.path) else {
        return Err(404);
    };

    if !route.methods.contains(&request.method) {
        return Err(405);
    }

    Ok(route)
}

fn redirect_response(target: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .into_bytes()
}

fn error_response(code: u16) -> Vec<u8> {
    let reason = reason(code);
    let body = format!(
        "<!DOCTYPE html>\n<html><head><title>{code} {reason}</title></head>\
         <body><h1>{code} {reason}</h1></body></html>"
    );
    build_response(code, "text/html; charset=utf-8", body.into_bytes())
}

fn build_response(code: u16, content_type: &str, body: Vec<u8>) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {code} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        reason(code),
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "Unknown",
    }
}
