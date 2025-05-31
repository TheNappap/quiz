
#[macro_use]
mod cli;
mod error;
mod server;
mod state;

use std::{net::SocketAddr, path::PathBuf};
use clap::Parser;


/// A simple quiz server app
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct AppArgs {
    /// The root of the server, a quiz.config (json) file should be located here.
    #[arg(name = "ROOT")]
    root: String,
    /// The ip address to bind to: Ipv4, Ipv6 or localhost.
    #[arg(long = "ip")]
    ip: Option<String>,
    /// The port to bind to.
    #[arg(long = "port")]
    port: Option<String>,
}

fn init() -> Result<(PathBuf, state::Config,SocketAddr),String> {
	let args = AppArgs::parse();

	let (root, config) = state::get_config(&args.root)?;
	Ok((
		root, config,
		server::get_socket(args.ip,args.port)?
	))
}

#[tokio::main]
async fn main() {
    match init() {
        Ok((root, config, socket)) => {
            println!("Starting quiz server in: {:?}", root);
            println!("Socket: {:?}", socket);

			let state = state::create_quiz_state(root, config);
			let sse = server::start(&state, socket).await;
			cli::start(state, sse).await;
        },
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
}
