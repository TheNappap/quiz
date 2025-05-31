
mod serve;
mod listener;
mod sse;

pub use sse::SseService;

use tokio::sync::mpsc::channel;
use std::net::SocketAddr;

use crate::state::QuizStateService;

pub fn get_socket(ip: Option<String>, port: Option<String>) -> Result<SocketAddr,String> {
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

pub async fn start(state: &QuizStateService, socket: SocketAddr) -> SseService  {
    let (job_sender, job_receiver) = channel(1000);
    sse::create_sse_state(job_receiver);
    let sse = SseService::new(job_sender);

    listener::start(state, &sse, socket).await;
    sse
}
