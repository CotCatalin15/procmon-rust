use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use kmum_common::{
    event::*, process::ProcessInformation, serializable_ntstring::SerializableNtString, *,
};
use nt_string::unicode_string::NtUnicodeString;
use procmon_core::communication::{CommunicationError, CommunicationInterface};
use rand::Rng;
use windows_sys::Win32::{
    Foundation::FILETIME, System::SystemInformation::GetSystemTimeAsFileTime,
};

pub struct FakeCommunication {
    stop_signal: AtomicBool,
}

impl CommunicationInterface for FakeCommunication {
    fn send_message_blocking(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, procmon_core::communication::CommunicationError>
    {
        match message {
            UmSendMessage::GetExeName(pid) => {
                let path = format!("Process{}.exe", pid);
                Ok(Some(KmReplyMessage::ExeName(SerializableNtString(
                    NtUnicodeString::try_from(&path).unwrap(),
                ))))
            }
            _ => Err(CommunicationError::Port),
        }
    }

    fn process_blocking<P: procmon_core::communication::EventProcessor>(&self, processor: P) {
        loop {
            if self.stop_signal.load(Ordering::Acquire) {
                break;
            }

            let events = Self::generate_random_events();
            tracing::info!("Generated {} number of new events", events.len());
            let mut iter = events.into_iter();
            let _ = processor.process(&mut iter);

            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn stop(&self) {
        self.stop_signal.store(true, Ordering::Release);
    }
}

impl FakeCommunication {
    pub fn new() -> Self {
        Self {
            stop_signal: AtomicBool::new(false),
        }
    }

    /// Generates a random number of `KmMessage` events
    pub fn generate_random_events() -> Vec<KmMessage> {
        let mut rng = rand::thread_rng();
        let num_events = rng.gen_range(100..=500); // Generate 1 to 10 events

        let mut events = Vec::with_capacity(num_events);

        for _ in 0..num_events {
            let pid = rng.gen_range(0..400); // Random PID between 1 and 30
            let unique_id = pid; // Set unique_id to the same value as pid

            let process_details = SimpleProcessDetails { pid, unique_id };

            let event_component = EventCompoent {
                date: get_system_time_as_file_time(), // Random date
                thread: rng.gen_range(1..100),        // Random thread ID
                operation: Self::generate_random_event_class(&mut rng),
                result: rng.gen_range(-1..=1), // Random result (-1, 0, or 1)
                path: SerializableNtString::new(NtUnicodeString::try_from("Some path").unwrap()), // Placeholder for path
                duration: rng.gen(), // Random duration
            };

            let event = KmMessage {
                event: event_component,
                process: process_details,
                stack: EventStack {}, // Placeholder for stack
            };

            events.push(event);
        }

        events
    }

    /// Generates a random `EventClass`
    fn generate_random_event_class<R: Rng>(rng: &mut R) -> EventClass {
        match rng.gen_range(0..=2) {
            0 => EventClass::Process(Self::generate_random_process_operation(rng)),
            1 => EventClass::FileSystem(Self::generate_random_filesystem_operation(rng)),
            2 => EventClass::Registry(EventRegistryOperation::Open()), // Placeholder for registry operations
            _ => unreachable!(),
        }
    }

    /// Generates a random `EventProcessOperation`
    fn generate_random_process_operation<R: Rng>(rng: &mut R) -> EventProcessOperation {
        match rng.gen_range(0..=1) {
            0 => EventProcessOperation::ProcessCreate {
                pid: rng.gen_range(1..=30),
                cmd: None, // Placeholder for command
            },
            1 => EventProcessOperation::ProcessDestroy {
                pid: rng.gen_range(1..=30),
            },
            _ => unreachable!(),
        }
    }

    /// Generates a random `EventFileSystemOperation`
    fn generate_random_filesystem_operation<R: Rng>(rng: &mut R) -> EventFileSystemOperation {
        match rng.gen_range(0..=3) {
            0 => EventFileSystemOperation::Create {
                attribute: rng.gen(),
            },
            1 => EventFileSystemOperation::Read {
                length: rng.gen(),
                offset: rng.gen(),
            },
            2 => EventFileSystemOperation::Write {
                length: rng.gen(),
                offset: rng.gen(),
            },
            3 => EventFileSystemOperation::Close {},
            _ => unreachable!(),
        }
    }
}

fn get_system_time_as_file_time() -> u64 {
    unsafe {
        // Create a FILETIME struct to hold the system time
        let mut file_time: FILETIME = std::mem::zeroed();

        // Call the Windows API to get the current system time
        GetSystemTimeAsFileTime(&mut file_time);

        // Combine the high and low parts of the FILETIME into a single u64
        ((file_time.dwHighDateTime as u64) << 32) | (file_time.dwLowDateTime as u64)
    }
}
