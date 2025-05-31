use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1::Builder as ConnectionBuilder;
use hyper::Method;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::pin::pin;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use crate::error::QuizResult;
use crate::state::QuizStateService;

use super::{serve, SseService};

pub async fn start(state: &QuizStateService, sse: &SseService, socket: SocketAddr) {
    let state = state.clone();
    let sse = sse.clone();
    tokio::task::spawn(async move {
        let listener = TcpListener::bind(socket).await.unwrap();
        handle_requests(state.clone(), sse.clone(), listener).await;
    });
}

async fn handle_requests(state: QuizStateService, sse: SseService, listener: TcpListener) {
    loop {
        let (tcp, _remote_address) = listener.accept().await.unwrap();
        //quiz_print!("accepted connection from {:?}", _remote_address);
        let io = TokioIo::new(tcp);

        let state_clone = state.clone();
        let sse_clone = sse.clone();
        tokio::task::spawn(async move {
            let service = hyper::service::service_fn(|req| {
                let state = state_clone.clone();
                let sse = sse_clone.clone();
                main_service(state,sse,req)
            });

            let conn = ConnectionBuilder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service);
            let mut connection = pin!(conn);
            
            if let Err(_e) = connection.as_mut().await {
                //quiz_print!("Server error: {}", _e);
            }
            connection.as_mut().graceful_shutdown();
        });
    }
}

async fn main_service(
    state: QuizStateService, 
    sse: SseService, 
    req: http::Request<Incoming>
) 
-> QuizResult<http::Response<serve::Body>> 
{
    let (parts, mut incoming) = req.into_parts();
    let body = incoming.frame().await
            .map(|frame| Full::new(frame.unwrap().into_data().unwrap()) );
    //quiz_print!("Request: {}",parts.uri.path());
    match (parts.method, parts.uri.path()) {
        (Method::POST, "/login")          => serve::login_answer(state,sse,body.unwrap()).await,
        (Method::POST, "/relogin")        => serve::relogin_answer(state,body.unwrap()).await,
        (Method::POST, "/submit_answer")  => serve::submit_answer(state,body.unwrap()).await,
        (Method::POST, "/last_event")     => serve::last_event(state,sse,body.unwrap()).await,
        (Method::GET, "/sse")             => serve::sse(sse).await,
        (Method::GET, "/title")           => serve::title(state).await,
        (Method::GET, file)         => serve::file(state,file.to_string()).await,
        (_,loc) => {
            quiz_print!("Unknown request: {}", loc);
            serve::not_found()
        }
    }
}