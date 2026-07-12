use crate::server::Server;

mod config;
mod server;
mod http;

fn main() {
    // first, parse the config file
    // then create the TcpListeners for each server
    // run the loop of listening to requests 
    match config::parse_config("./config.conf") {
        Ok(configs) => {
            match Server::bind(configs) {
                Ok(mut server) => server.run(),
                Err(e) =>panic!("FOR SOME REASON PANICKED{e}"),
            };
        }
        Err(e) => panic!("{}", format!("error parsing your config file: {}", e)),
    }

}
