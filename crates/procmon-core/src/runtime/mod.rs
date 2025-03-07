use std::sync::Arc;

use kmum_common::KmMessage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    spawn,
    sync::mpsc::channel,
    task::{spawn_blocking, JoinSet},
};

use crate::communication::{communication::Communication, CommunicationError};

pub struct ProcmonRuntime {
    communication: Arc<Communication>,
}

impl ProcmonRuntime {
    pub fn new() -> Self {
        Self {
            communication: Arc::new(Communication::new()),
        }
    }

    pub async fn run(&self) {
        //process events and stuff

        let mut handlees = JoinSet::new();

        for _ in 0..6 {
            let comm = self.communication.clone();
            let (sender, mut receiver) = channel(512);
            handlees.spawn_blocking(move || {
                comm.process_blocking(|iter| {
                    for msg in iter {
                        sender
                            .blocking_send(msg)
                            .map_err(|_| CommunicationError::TokioSender)?;
                    }
                    Ok(())
                });
            });

            handlees.spawn(async {
                let mut file = tokio::fs::File::open("test.txt").await.unwrap();

                let mut msg = [0u8; 1024];
                file.write_all("Hello".as_bytes()).await.unwrap();
                file.read(&mut msg).await.unwrap();

                drop(file);
            });

            handlees.spawn(async move {
                let mut recv_buffer: Vec<KmMessage> = Vec::new();
                loop {
                    let size = receiver.recv_many(&mut recv_buffer, 512).await;
                    if size == 0 {
                        break;
                    }

                    let messages = &recv_buffer[..size];
                    for msg in messages {
                        if msg.process.pid == std::process::id() as u64 {
                            dbg!(msg);
                        }
                    }
                }
            });
        }

        handlees.join_all().await;
    }
}
