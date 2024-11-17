use core::time::Duration;

use kmum_common::KmMessage;
use maple::info;
use nt_string::unicode_string::NtUnicodeString;
use wdrf::{
    context::{Context, FixedGlobalContextRegistry},
    logger::DbgPrintLogger,
    minifilter::FltFilter,
};
use wdrf_std::{
    constants::PoolFlags,
    kmalloc::{GlobalKernelAllocator, MemoryTag},
    sync::{arc::Arc, event::Event},
    sys::{event::EventType, WaitResponse, WaitableObject},
    thread::{spawn, JoinHandle},
    time::Timeout,
};

use crate::communication::Communication;

#[global_allocator]
pub static KERNEL_GLOBAL_ALLOCATOR: GlobalKernelAllocator = GlobalKernelAllocator::new(
    MemoryTag::new_from_bytes(b"allc"),
    PoolFlags::POOL_FLAG_NON_PAGED,
);

pub static CONTEXT_REGISTRY: FixedGlobalContextRegistry<10> = FixedGlobalContextRegistry::new();

pub static DBGPRINT_LOGGER: Context<DbgPrintLogger> = Context::uninit();

#[allow(dead_code)]
pub struct Test {
    stop_event: Arc<Event>,
    thread: JoinHandle<()>,
}

impl Test {
    pub fn new() -> Self {
        let event = Event::try_create_arc(EventType::Notification, false).unwrap();
        Self {
            stop_event: event.clone(),
            thread: spawn(move || Self::run_worker(event)).unwrap(),
        }
    }

    fn run_worker(event: Arc<Event>) {
        let communication = &DRIVER_CONTEXT.get().communication;

        loop {
            if event.wait_for(Duration::from_secs(5)) == WaitResponse::Success {
                return;
            }

            {
                let msg = KmMessage::WriteFile(
                    NtUnicodeString::try_from("Alabalaportocolala")
                        .unwrap()
                        .into(),
                );
                let result = communication.send_with_reply(&msg, Timeout::infinite());
                match result {
                    Ok(reply) => {
                        reply.inspect(|reply| {
                            info!("Received reply: {:#?}", reply);
                        });
                    }
                    Err(_) => continue,
                }
            }

            {
                let msg = KmMessage::CreateFile(
                    NtUnicodeString::try_from("C:/Ladialadladladas")
                        .unwrap()
                        .into(),
                );
                let result = communication.send_with_reply(&msg, Timeout::infinite());
                match result {
                    Ok(reply) => {
                        reply.inspect(|reply| {
                            info!("Received reply: {:#?}", reply);
                        });
                    }
                    Err(_) => continue,
                }
            }
        }
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        self.stop_event.signal();
    }
}

unsafe impl Send for Test {}

pub struct DriverContext {
    pub filter: FltFilter,
    pub communication: Communication,
    pub test: Option<Test>,
}

//unsafe impl Send for DriverContext {}
//unsafe impl Sync for DriverContext {}

pub static DRIVER_CONTEXT: Context<DriverContext> = Context::uninit();
