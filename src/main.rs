
#[macro_use]
mod cli;
mod error;
mod service;
mod sse;
mod quiz_state;
mod question;

use sse::SSE;
use tokio::sync::RwLock;
use std::{net::SocketAddr, sync::Arc};
use clap::Parser;

fn get_state(root: &str) -> Result<quiz_state::QuizState,String> {
	let path = root.to_string();
	std::fs::canonicalize(&path)
		.map_err(|_| format!("Could not find quiz root: {}\n", path))
		.and_then(|root|{
			quiz_state::QuizState::new(root)
				.map_err(|e| format!("Could not import quiz.config file: {}\n", e))
		})
}

fn get_socket(ip: Option<String>, port: Option<String>) -> Result<SocketAddr,String> {
	let ip = if ip.is_none() {
			let ip : Result<_,String> = local_ipaddress::get()
				.ok_or("Could not retrieve local ip address\n".into());
			Some(ip?)
		} else { 
			ip.map(|ip| 
				if ip == "localhost" { "127.0.0.1".into() }
				else { ip.to_string() }
			) 
		};
	let port = port.unwrap_or("80".into());
	
	let ip = ip.clone().unwrap().parse()
		.map_err(|_| format!("Could not parse ip address: {:?}", ip))?;
	let port = port.parse()
		.map_err(|_| format!("Could not parse port: {:?}", port))?;
	Ok(SocketAddr::new(ip,port))
}

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

fn init() -> Result<(quiz_state::QuizState,SocketAddr),String> {
	let args = AppArgs::parse();

	Ok((
		get_state(&args.root)?, 
		get_socket(args.ip,args.port)?
	))
}

#[tokio::main]
async fn main() {
    match init() {
        Ok((state,socket)) => {
            println!("Starting quiz server in: {:?}", state.root());
            println!("Socket: {:?}", socket);
			let state = Arc::new(RwLock::new(state));
			let sse = Arc::new(RwLock::new(SSE::new()));
			let state_clone = state.clone();
			let sse_clone = sse.clone();
			tokio::task::spawn(async move {
				service::main(state.clone(), sse.clone(), socket).await;
			});
			cli::main(state_clone, sse_clone).await;
        },
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
}
