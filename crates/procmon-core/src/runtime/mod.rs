use std::{future::Future, sync::Arc};

use kmum_common::KmMessage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinSet,
};

use crate::communication::{CommunicationError, CommunicationInterface, EventProcessor};

pub struct ProcmonRuntime<C: CommunicationInterface> {
    communication: Arc<C>,
}

impl<C: CommunicationInterface> ProcmonRuntime<C> {
    pub fn new(communication: C) -> Self {
        Self {
            communication: Arc::new(communication),
        }
    }

    pub async fn run<F, Fut>(self, async_consumer: F)
    where
        F: FnOnce(Receiver<KmMessage>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        //process events and stuff

        let mut handles = JoinSet::new();

        tracing::info!("Starting runtime engine");

        let (sender, receiver) = channel(512);
        for _ in 0..6 {
            let comm = self.communication.clone();
            let sender_clone = sender.clone();
            handles.spawn_blocking(move || {
                comm.process_blocking(RuntimeEventProcessor {
                    sender: sender_clone,
                });
            });
        }
        handles.spawn(async_consumer(receiver));

        handles.join_all().await;
    }
}

struct RuntimeEventProcessor {
    sender: Sender<KmMessage>,
}

impl EventProcessor for RuntimeEventProcessor {
    fn process<I>(&self, iter: &mut I) -> anyhow::Result<(), CommunicationError>
    where
        I: Iterator<Item = KmMessage>,
    {
        for msg in iter {
            self.sender
                .blocking_send(msg)
                .map_err(|_| CommunicationError::TokioSender)?;
        }
        Ok(())
    }
}
