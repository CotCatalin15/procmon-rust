use kmum_common::{
    process::{ProcessInformation, UniqueProcessId},
    KmMessage, UmSendMessage,
};
use procmon_core::communication::{
    driver_communication::DriverCommunication, CommunicationInterface, EventProcessor,
};
use std::{
    process::{Child, Command},
    sync::Arc,
};
use tokio::task::spawn_blocking;

use crate::{
    events_storage::EventStorage, fake_communication::FakeCommunication,
    process_cache::ProcessCache, ProcmonArgs,
};

pub struct ClientRuntime {
    internal: Box<dyn ClientRuntimeInterface>,
    num_threads: u32,
    child_process: Option<Child>,
    cache: Arc<ProcessCache>,
}

impl ClientRuntime {
    pub fn from_args(storage: EventStorage, args: &ProcmonArgs) -> Self {
        let mut tester = None;
        let b: Box<dyn ClientRuntimeInterface> = match args.communication {
            crate::CommunicationType::Driver => {
                Box::new(InternalRuntime::new(DriverCommunication::new(), storage))
            }
            crate::CommunicationType::Fake => {
                Box::new(InternalRuntime::new(FakeCommunication::new(), storage))
            }
            crate::CommunicationType::DriverTest => {
                let child_proc = Command::new("procmon-tester.exe").spawn().unwrap();
                let id = child_proc.id();
                tester = Some(child_proc);

                Box::new(InternalRuntime::new(
                    DriverCommunication::new_test(id as _),
                    storage,
                ))
            }
        };

        let cache = b.create_cache();
        Self {
            internal: b,
            num_threads: args.num_threads.get(),
            child_process: tester,
            cache: cache,
        }
    }

    pub fn start(&self) {
        self.internal.start(self.num_threads);
    }
    pub fn stop(&self) {
        self.internal.stop();
    }

    pub fn cache(&self) -> &ProcessCache {
        &self.cache
    }
}

impl Drop for ClientRuntime {
    fn drop(&mut self) {
        if let Some(mut c) = self.child_process.take() {
            c.kill();
        }

        self.internal.stop();
    }
}

trait ClientRuntimeInterface {
    fn start(&self, num_threads: u32);
    fn stop(&self);

    fn create_cache(&self) -> Arc<ProcessCache>;
}

struct InternalRuntime<C: CommunicationInterface> {
    communication: Arc<C>,
    storage: EventStorage,
}

impl<C: CommunicationInterface> InternalRuntime<C> {
    fn new(communication: C, storage: EventStorage) -> Self {
        Self {
            communication: Arc::new(communication),
            storage,
        }
    }
}

impl<C: CommunicationInterface> ClientRuntimeInterface for InternalRuntime<C> {
    fn start(&self, num_threads: u32) {
        struct Processor {
            storage: EventStorage,
        }
        impl EventProcessor for Processor {
            fn process<I>(
                &self,
                iter: &mut I,
            ) -> anyhow::Result<(), procmon_core::communication::CommunicationError>
            where
                I: Iterator<Item = kmum_common::KmMessage>,
            {
                self.storage.push_received(iter);
                Ok(())
            }
        }

        for i in 0..num_threads {
            let communication_clone = self.communication.clone();
            let storage_clone = self.storage.clone();
            spawn_blocking(move || {
                let processor = Processor {
                    storage: storage_clone,
                };
                communication_clone.process_blocking(processor);
            });
        }
    }

    fn stop(&self) {
        self.communication.stop();
    }

    fn create_cache(&self) -> Arc<ProcessCache> {
        let communication = self.communication.clone();
        ProcessCache::new(move |id| {
            let msg = communication
                .send_message_blocking(&UmSendMessage::GetExeName(id))
                .unwrap_or_else(|_| None);

            if let Some(reply) = msg {
                match reply {
                    kmum_common::KmReplyMessage::ExeName(name) => Some(name),
                    _ => {
                        tracing::error!(
                            "Received other type of reply instead of process information"
                        );
                        None
                    }
                }
            } else {
                None
            }
        })
    }
}
