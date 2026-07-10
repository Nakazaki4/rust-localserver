mod config;
mod server;
fn main() {
    // first, parse the config file
    match config::parse_config("./config.conf") {
        Ok(servers) => 
        servers.iter().for_each(|s| {
            server::
        })
        ,
        Err(e) => panic!("{}", format!("an error occured: {e}"))
    }
    
    // then create the TcpListeners for each server
}
