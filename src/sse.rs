
use hyper::body::{Sender,Bytes};

pub struct SSE {
    clients: Vec<hyper::body::Sender>,
    last_event: String
}

impl SSE {
    pub fn new() -> Self {
        SSE { 
            clients: Vec::new(), 
            last_event: "null".into()
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
            async move { client.send_data(bytes).await.is_ok() }
        })).await.into_iter();
        self.clients.retain(|_| sent.next().unwrap());
        self.clients.len()
    }

    pub fn last_event(&self) -> String {
        self.last_event.clone()
    }
    
    pub fn close(&mut self) {
        for client in std::mem::replace(&mut self.clients, Vec::new()) {
            client.abort();
        }
    }
}