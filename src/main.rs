use crate::server::Server;

mod config;
mod server;
fn main() {
    // first, parse the config file
    // then create the TcpListeners for each server
    // run the loop of listening to requests 
    match config::parse_config("./config.conf") {
        Ok(servers) => {
            servers.iter().enumerate().for_each(
                |(index, server_conf)| {
                Server::bind(server_conf.clone(), index).unwrap();
            });
        }
        Err(e) => panic!("{}", format!("error parsing your config file: {}", e)),
    }

}
