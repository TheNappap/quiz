
use hyper::body::{Bytes, Frame};
use tokio::sync::mpsc;
use crate::error::Error;

type Sender = mpsc::Sender<http::Result<Frame<Bytes>>>;

pub struct SSE {
    clients: Vec<Sender>,
    last_event: String,
    error_log: Vec<Error>,
}

impl SSE {
    pub fn new() -> Self {
        SSE { 
            clients: Vec::new(), 
            last_event: "null".into(),
            error_log: Vec::new(),
        }
    }

    pub fn add_client(&mut self, client: Sender) {
        self.clients.push(client);
    }

    pub async fn send_to_clients<S: Into<String>>(&mut self, text: S) -> usize {
        let event = text.into();
        self.last_event = event.clone();
        let bytes: Bytes = format!("data:{}\n\n", event).into();
        let mut sent = futures::future::join_all(self.clients.iter_mut().map(|client| {
            let bytes = bytes.slice(..);
            async move { client.send(Ok(Frame::data(bytes))).await.is_ok() }
        })).await.into_iter();
        self.clients.retain(|_| sent.next().unwrap());
        self.clients.len()
    }

    pub fn last_event(&self) -> String {
        self.last_event.clone()
    }
    
    pub async fn close(&mut self) {
        // TODO send end quiz final event
    }

    pub fn log_error(&mut self, e: Error) {
        self.error_log.push(e);
    }

    pub fn errors(&self) -> &Vec<Error> {
        &self.error_log
    }

}