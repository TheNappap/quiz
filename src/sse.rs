
use hyper::body::{Bytes, Frame};
use tokio::sync::mpsc;
use crate::{error::Error, quiz_state::Event};

type Sender = mpsc::Sender<http::Result<Frame<Bytes>>>;

pub struct SSE {
    clients: Vec<Sender>,
    last_event: Option<Event>,
    error_log: Vec<Error>,
}

impl SSE {
    pub fn new() -> Self {
        SSE { 
            clients: Vec::new(), 
            last_event: None,
            error_log: Vec::new(),
        }
    }

    pub fn add_client(&mut self, client: Sender) {
        self.clients.push(client);
    }

    pub async fn send_to_clients(&mut self, event: Event) {
        let event_json = event.to_string();
        self.last_event = Some(event);
        
        let bytes: Bytes = format!("data:{}\n\n", event_json).into();
        let mut sent = futures::future::join_all(self.clients.iter_mut().map(|client| {
            let bytes = bytes.slice(..);
            async move { client.send(Ok(Frame::data(bytes))).await.is_ok() }
        })).await.into_iter();

        // remove unresponsive clients
        self.clients.retain(|_| sent.next().unwrap());
    }

    pub fn last_event(&self) -> Option<Event> {
        self.last_event.clone()
    }
    
    pub async fn close(&mut self) {
        self.send_to_clients(Event::Closed).await;
        self.clients.clear();
    }

    pub fn log_error(&mut self, e: Error) {
        self.error_log.push(e);
    }

    pub fn errors(&self) -> &Vec<Error> {
        &self.error_log
    }

}