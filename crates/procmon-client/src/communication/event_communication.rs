use std::sync::Arc;

use flume::Sender;
use kmum_common::KmMessage;
use procmon_core::communication::{CommunicationInterface, EventProcessor};
use rayon::ThreadPoolBuilder;

pub struct EventCommunication {
    stop_function: Box<dyn Fn()>,
}

impl EventCommunication {
    pub fn new<C: CommunicationInterface>(
        event_sender: Sender<KmMessage>,
        communication: Arc<C>,
        num_threads: usize,
    ) -> Self {
        for i in 0..num_threads {
            let c_clone = communication.clone();
            let sender_clone = event_sender.clone();

            rayon::spawn(move || {
                Self::communication_routine(c_clone, sender_clone);
            });
        }

        Self {
            stop_function: Box::new(move || {
                communication.stop();
            }),
        }
    }

    pub fn stop(&self) {
        (self.stop_function)();
    }

    fn communication_routine<C: CommunicationInterface>(
        communication: Arc<C>,
        sender: Sender<KmMessage>,
    ) {
        struct MyProcessor {
            sender: Sender<KmMessage>,
        }

        impl EventProcessor for MyProcessor {
            fn process<I>(
                &self,
                iter: &mut I,
            ) -> anyhow::Result<(), procmon_core::communication::CommunicationError>
            where
                I: Iterator<Item = KmMessage>,
            {
                for event in iter {
                    let _ = self.sender.send(event);
                }

                Ok(())
            }
        }

        communication.process_blocking(MyProcessor { sender: sender });
    }
}

impl Drop for EventCommunication {
    fn drop(&mut self) {
        (self.stop_function)();
    }
}
