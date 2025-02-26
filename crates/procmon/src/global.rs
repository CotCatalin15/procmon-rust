use wdrf::{
    context::{Context, FixedGlobalContextRegistry},
    logger::DbgPrintLogger,
};
use wdrf_std::{
    constants::PoolFlags,
    kmalloc::{GlobalKernelAllocator, MemoryTag},
    sync::{arc::Arc, event::Event},
    thread::JoinHandle,
};

use crate::{communication::Communication, pscollector::ProcessCollectorCache};

#[global_allocator]
pub static KERNEL_GLOBAL_ALLOCATOR: GlobalKernelAllocator = GlobalKernelAllocator::new(
    MemoryTag::new_from_bytes(b"allc"),
    PoolFlags::POOL_FLAG_NON_PAGED,
);

pub static CONTEXT_REGISTRY: FixedGlobalContextRegistry<10> = FixedGlobalContextRegistry::new();

pub static DBGPRINT_LOGGER: Context<DbgPrintLogger> = Context::uninit();

pub struct DriverContext {
    pub communication: Communication,
    pub stop_event: Arc<Event>,
    pub process_cache: ProcessCollectorCache,
    pub test_thread: Option<JoinHandle<()>>,
}

pub static DRIVER_CONTEXT: Context<DriverContext> = Context::uninit();
