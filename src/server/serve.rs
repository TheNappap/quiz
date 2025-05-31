use http_body_util::combinators::BoxBody;
    use http_body_util::{BodyExt, Full, StreamBody};
    use hyper::body::Bytes;
    use hyper::{Response,Request,StatusCode};
    use tokio::sync::mpsc::channel;
    use tokio_stream::wrappers::ReceiverStream;
    use crate::error::{Error, IntoQuizResult, QuizResult};
    use crate::state::QuizStateService;

    use super::SseService;

    pub type Body = BoxBody<Bytes, Error>;
    
    fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
        Full::new(chunk.into())
            .map_err(|e| http::Error::from(e).into())
            .boxed()
    }

    pub fn not_found() -> QuizResult<Response<Body>> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(full("404 NOT FOUND"))
            .into_result()
    }

    pub async fn file(state: QuizStateService, path: String) -> QuizResult<Response<Body>> {
        let root = state.root().await;
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

    pub async fn sse(sse: SseService) -> QuizResult<Response<Body>> {
        let (send, receiver) = channel(1000);
        let body = StreamBody::new(ReceiverStream::new(receiver));
        let boxed_body = body
            .map_err(|e| Error::from(e))
            .boxed();

        sse.add_client(send).await;

        Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(boxed_body)
            .into_result()
    }

    pub async fn title(state: QuizStateService) -> QuizResult<Response<Body>> {
        let title = state.title().await;
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(full(title))
            .into_result()
    }

    pub async fn login_answer(state: QuizStateService, sse: SseService, body: Full<Bytes>) -> QuizResult<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.add_user(&username).await.is_ok() {
                if let Some(e) = state.lobby().await {
                    sse.send_event(e).await;
                }
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(username))
                    .into_result();
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Invalid username"))
            .into_result()
    }

    pub async fn relogin_answer(state: QuizStateService, body: Full<Bytes>) -> QuizResult<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.user_exists(&username).await {
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(username))
                    .into_result();
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Could not relogin"))
            .into_result()
    }

    pub async fn submit_answer(state: QuizStateService, body: Full<Bytes>) -> QuizResult<Response<Body>> {
        if let Some(answer) = to_string(body).await {
            let answer = serde_json::from_str(&answer)?;
            match state.submit_answer(&answer).await {
                Ok(answer) => return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header("Content-Type", "text/plain")
                    .body(full(answer))
                    .into_result(),
                Err(err) => return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(full(err))
                    .into_result(),
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Answer was not accepted"))
            .into_result()
    }

    pub async fn last_event(state: QuizStateService, sse: SseService, body: Full<Bytes>) -> QuizResult<Response<Body>> {
        if let Some(username) = to_string(body).await {
            if state.user_exists(&username).await {
                let last_event_json = match sse.last_event().await {
                    None => "null".to_string(),
                    Some(event) => event.to_string(),
                };
                return Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(full(last_event_json))
                    .into_result();
            } else {
                quiz_print!("Unknown user request: {}", username)
            }
        }
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(full("Unknown user"))
            .into_result()
    }

    async fn to_string(mut body: Full<Bytes>) -> Option<String> {
        let bytes = body.frame().await.unwrap().unwrap().into_data().unwrap();
        Some(std::str::from_utf8(bytes.as_ref()).ok()?.to_string())
    }