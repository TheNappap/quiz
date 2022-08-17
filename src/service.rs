use hyper::{Body,Method};
use std::convert::Infallible;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use crate::quiz_state::QuizState;
use crate::sse::SSE;

pub async fn main(state: QuizState, socket: SocketAddr) {
    let state = Arc::new(RwLock::new(state));
    let sse = Arc::new(RwLock::new(SSE::new()));
    
    let make_service = hyper::service::make_service_fn(|_conn| {
        let state = state.clone();
        let sse = sse.clone();
        async move {
            Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                let state = state.clone();
                let sse = sse.clone();
                main_service(state,sse,req)
            }))
        }
    });

    let server = hyper::Server::bind(&socket)
        .serve(make_service)
        .with_graceful_shutdown(super::cli::main(state.clone(),sse.clone()));

    if let Err(e) = server.await {
        quiz_print!("Server error: {}", e);
    }
}

async fn main_service(
    state: Arc<RwLock<QuizState>>, 
    sse: Arc<RwLock<SSE>>, 
    req: http::Request<Body>
) 
-> Result<http::Response<Body>, Infallible> 
{
    let (parts, body) = req.into_parts();
    //quiz_print!("Request: {}",parts.uri.path());
    Ok(match (parts.method, parts.uri.path()) {
        (Method::POST, "/login")        => serve::login_answer(state,sse,body).await,
        (Method::POST, "/relogin")      => serve::relogin_answer(state,body).await,
        (Method::POST, "/submit_answer") => serve::submit_answer(state,body).await,
        (Method::POST, "/last_event")    => serve::last_event(state,sse,body).await,
        (Method::GET, "/sse")           => serve::sse(sse).await,
        (Method::GET, "/title")         => serve::title(state).await,
        (Method::GET, file)             => serve::file(state,file.to_string()).await,
        (_,loc) => {
            quiz_print!("Unknown request: {}", loc);
            serve::not_found()
        }
    }.expect("could not build response"))
}

mod serve {
    use hyper::{Body,Response,Request,StatusCode};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use crate::quiz_state::QuizState;
    use crate::sse::SSE;

    pub fn not_found() -> http::Result<Response<Body>> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(Body::from("404 NOT FOUND"))
    }

    pub async fn file(state: Arc<RwLock<QuizState>>, path: String) -> http::Result<Response<Body>> {
        let root = state.read().await.root().clone();
        let req = Request::get(path).body(Body::empty()).unwrap();
        match hyper_staticfile::Static::new(root).serve(req).await {
            Ok(f) => Ok(f),
            Err(e) => {
                quiz_print!("Could not serve file: {}", e);
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("404 NOT FOUND"));
            }
        }
    }

    pub async fn sse(sse: Arc<RwLock<SSE>>) -> http::Result<Response<Body>> {
        let (channel, body) = Body::channel();
        sse.write().await.add_client(channel);
        Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(body)
    }

    pub async fn title(state: Arc<RwLock<QuizState>>) -> http::Result<Response<Body>> {
        let title = state.read().await.title().clone();
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(title.into())
    }

    pub async fn login_answer(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, body: Body) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.write().await.add_user(username.clone()).is_ok() {
                if let Some(e) = state.read().await.lobby().and_then(|e|e.to_string().ok()) {
                    sse.write().await.send_to_clients(e).await;
                }
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(username.into());
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Body::from("Invalid username"))
    }

    pub async fn relogin_answer(state: Arc<RwLock<QuizState>>, body: Body) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.read().await.user_exists(&username) {
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(username.into());
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Body::from("Could not relogin"))
    }

    pub async fn submit_answer(state: Arc<RwLock<QuizState>>, body: Body) -> http::Result<Response<Body>> {
        if let Some(answer) = to_string(body).await {
            match state.write().await.submit_answer(&answer) {
                Ok(Ok(answer)) => return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(answer.into()),
                Ok(Err(err)) => return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(err.into()),
                Err(e) => quiz_print!("Submit error: {:?}",e)
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Body::from("Answer was not accepted"))
    }

    pub async fn last_event(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, body: Body) -> http::Result<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.read().await.user_exists(&username) {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(sse.read().await.last_event().into());
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Body::from("Unknown user"))
    }

    async fn to_string(body: Body) -> Option<String> {
        let bytes = hyper::body::to_bytes(body).await.ok()?;
        Some(std::str::from_utf8(bytes.as_ref()).ok()?.to_string())
    }
}

