use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1::Builder as ConnectionBuilder;
use hyper::Method;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::pin::pin;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use crate::quiz_state::QuizState;
use crate::sse::SSE;

pub async fn main(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, socket: SocketAddr) {
    let listener = TcpListener::bind(socket).await.unwrap();
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
            
            if let Err(e) = connection.as_mut().await {
                quiz_print!("Server error: {}", e);
            }
            connection.as_mut().graceful_shutdown();
        });

    }
}

async fn main_service(
    state: Arc<RwLock<QuizState>>, 
    sse: Arc<RwLock<SSE>>, 
    req: http::Request<Incoming>
) 
-> Result<http::Response<serve::Body>, Infallible> 
{
    let (parts, mut incoming) = req.into_parts();
    let body = incoming.frame().await
            .map(|frame| Full::new(frame.unwrap().into_data().unwrap()) );
    //quiz_print!("Request: {}",parts.uri.path());
    Ok(match (parts.method, parts.uri.path()) {
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
    }.expect("could not build response"))
}


mod serve {
    use http_body_util::combinators::BoxBody;
    use http_body_util::{BodyExt, Full, StreamBody};
    use hyper::body::Bytes;
    use hyper::{Response,Request,StatusCode};
    use tokio_stream::wrappers::ReceiverStream;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};
    use crate::error::Error;
    use crate::quiz_state::QuizState;
    use crate::sse::SSE;

    pub type Body = BoxBody<Bytes, Error>;
    
    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
        Full::new(chunk.into())
            .map_err(|e| http::Error::from(e).into())
            .boxed()
    }

    pub fn not_found() -> http::Result<Response<Body>> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(full("404 NOT FOUND"))
    }

    pub async fn file(state: Arc<RwLock<QuizState>>, path: String) -> http::Result<Response<Body>> {
        let root = state.read().await.root().clone();
        let request = Request::get(path).body(())?;
        match hyper_staticfile::Static::new(root).serve(request).await {
            Ok(response) => {
                let (parts, body) = response.into_parts();
                let boxed_body = body
                    .map_err(|e|e.into())
                    .boxed();
                return Ok(Response::from_parts(parts, boxed_body));
            },
            Err(e) => {
                quiz_print!("Could not serve file: {}", e);
                return not_found();
            }
        }
    }

    pub async fn sse(sse: Arc<RwLock<SSE>>) -> http::Result<Response<Body>> {
        let (send, receiver) = mpsc::channel(1000);
        let body = StreamBody::new(ReceiverStream::new(receiver));
        let boxed_body = body
            .map_err(|e| Error::from(e))
            .boxed();

        sse.write().await.add_client(send);
        Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(boxed_body)
    }

    pub async fn title(state: Arc<RwLock<QuizState>>) -> http::Result<Response<Body>> {
        let title = state.read().await.title().clone();
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(full(title))
    }

    pub async fn login_answer(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, body: Full<Bytes>) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.write().await.add_user(username.clone()).is_ok() {
                if let Some(e) = state.read().await.lobby().and_then(|e|e.to_string().ok()) {
                    sse.write().await.send_to_clients(e).await;
                }
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(username));
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Invalid username"))
    }

    pub async fn relogin_answer(state: Arc<RwLock<QuizState>>, body: Full<Bytes>) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.read().await.user_exists(&username) {
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(username));
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Could not relogin"))
    }

    pub async fn submit_answer(state: Arc<RwLock<QuizState>>, body: Full<Bytes>) -> http::Result<Response<Body>> {
        if let Some(answer) = to_string(body).await {
            match state.write().await.submit_answer(&answer) {
                Ok(Ok(answer)) => return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(answer)),
                Ok(Err(err)) => return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(full(err)),
                Err(e) => quiz_print!("Submit error: {:?}",e)
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Answer was not accepted"))
    }

    pub async fn last_event(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, body: Full<Bytes>) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.read().await.user_exists(&username) {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(full(sse.read().await.last_event()));
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Unknown user"))
    }

    async fn to_string(mut body: Full<Bytes>) -> Option<String> {
        let bytes = body.frame().await.unwrap().unwrap().into_data().unwrap();
        Some(std::str::from_utf8(bytes.as_ref()).ok()?.to_string())
    }
}

