
use hyper::body::{Bytes, Frame};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot::{self, Sender as Return};
use crate::{error::Error, state::Event};

type Client = Sender<http::Result<Frame<Bytes>>>;

pub enum SseJob {
    AddClient(Client),
    SendEvent(Event),
    LastEvent(Return<Option<Event>>),
    Close,
}

#[derive(Debug, Clone)]
pub struct SseService {
    job_channel: Sender<SseJob>
}

impl SseService {
    pub fn new(job_channel: Sender<SseJob>) -> Self {
        SseService { job_channel }
    }

    pub async fn add_client(&self, client: Client) {
        self.job_channel.send(SseJob::AddClient(client)).await.expect("Send failed");
    }

    pub async fn send_event(&self, event: Event) {
        self.job_channel.send(SseJob::SendEvent(event)).await.expect("Send failed");
    }

    pub async fn last_event(&self) -> Option<Event> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(SseJob::LastEvent(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn close(&self) {
        self.job_channel.send(SseJob::Close).await.expect("Send failed");
    }
}

pub fn create_sse_state(job_receiver: Receiver<SseJob>) {            
    tokio::task::spawn(async move {
        let sse = SseState { 
            clients: Vec::new(), 
            last_event: None,
            _error_log: Vec::new(),
        };

        sse.handle_jobs(job_receiver).await
    });
}

struct SseState {
    clients: Vec<Client>,
    last_event: Option<Event>,
    _error_log: Vec<Error>,
}

impl SseState {
    async fn handle_jobs(mut self, mut job_receiver: Receiver<SseJob>) {
        loop {
            if let Some(job) = job_receiver.recv().await {
                match job {
                    SseJob::AddClient(client) => self.add_client(client),
                    SseJob::SendEvent(event) => self.send_to_clients(event).await,
                    SseJob::LastEvent(callback) => callback.send(self.last_event()).expect("Failed returning last event."),
                    SseJob::Close => {
                        self.close().await;
                        break;
                    }
                }
            }
        }
    }

    fn add_client(&mut self, client: Client) {
        self.clients.push(client);
    }

    async fn send_to_clients(&mut self, event: Event) {
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

    fn last_event(&self) -> Option<Event> {
        self.last_event.clone()
    }
    
    async fn close(&mut self) {
        self.send_to_clients(Event::Closed).await;
        self.clients.clear();
    }

}