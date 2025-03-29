use kmum_common::{
    event::{EventClass, EventCompoent, EventProcessOperation, EventStack, SimpleProcessDetails},
    process::{ProcessInformation, UniqueProcessId},
    serializable_ntstring::SerializableNtString,
    KmMessage,
};
use wdrf::process::{
    notifier::PsNotifierRegistration, process_create_notifier::PsCreateNotifyCallback,
    ps_lookup_by_process_id, PsCreateNotifyInfo,
};
use wdrf_std::{
    boxed::Box,
    dbg_break,
    kmalloc::TaggedObject,
    nt_success,
    object::ArcKernelObj,
    structs::PKPROCESS,
    time::SystemTime,
    vec::{Vec, VecCreate, VecExt},
};
use windows_sys::{
    Wdk::System::SystemServices::PsGetCurrentThreadId,
    Win32::{
        Foundation::{STATUS_INFO_LENGTH_MISMATCH, STATUS_SUCCESS},
        System::WindowsProgramming::SYSTEM_PROCESS_INFORMATION,
    },
};

use crate::{
    global::DRIVER_CONTEXT,
    imports::{DYN_IMPORTS, SYSTEM_PROCESS_INFORMATION_CLASS},
};

use super::{collection::PsInfoContainer, proc_info_factory::ProcessInformationFactory};

pub struct ProcessCollectorCache {
    container: PsInfoContainer,
    ps_notifier: PsNotifierRegistration<CacheNotifierCallback>,
}

unsafe impl Send for ProcessCollectorCache {}
unsafe impl Sync for ProcessCollectorCache {}

impl ProcessCollectorCache {
    pub fn try_create() -> anyhow::Result<Self> {
        let ps_notifier = PsNotifierRegistration::try_create(CacheNotifierCallback {})
            .map_err(|_| anyhow::Error::msg("Failed to create collector cache"))?;

        Ok(Self {
            container: PsInfoContainer::create(),
            ps_notifier,
        })
    }

    pub fn try_start(&self) -> anyhow::Result<()> {
        self.ps_notifier
            .try_start()
            .map_err(|_| anyhow::Error::msg("Failed to start process collector cache"))?;

        self.scan_unmonitored()
    }

    pub fn try_stop(&self) -> anyhow::Result<()> {
        self.ps_notifier
            .try_stop()
            .map_err(|_| anyhow::Error::msg("Failed to stop process collector cache"))
    }

    pub fn get_process_info_from_uid(
        &self,
        unique_id: UniqueProcessId,
    ) -> Option<ProcessInformation> {
        self.container.get_info(unique_id)
    }

    pub fn pid_to_unique_id(&self, pid: u64) -> Option<UniqueProcessId> {
        self.container.get_uid(pid)
    }

    fn internal_on_process_create(
        &self,
        eprocess: &ArcKernelObj<PKPROCESS>,
        pid: u64,
        process_info: &PsCreateNotifyInfo,
    ) -> UniqueProcessId {
        self.container.map_pid(
            pid,
            ProcessInformationFactory::try_create_from_process_create(
                eprocess,
                pid as _,
                process_info,
            ),
        )
    }

    fn internal_on_process_exit(&self, pid: u64) -> Option<UniqueProcessId> {
        self.container.unmap_pid(pid)
    }

    fn scan_unmonitored(&self) -> anyhow::Result<()> {
        let mut buffer: Vec<u8> = Vec::create();
        buffer.try_resize(1024 * 4, 0)?;

        unsafe {
            dbg_break();
        }

        let mut returned_length = 0;
        loop {
            let status = unsafe {
                DYN_IMPORTS.get().zw_query_system_information(
                    SYSTEM_PROCESS_INFORMATION_CLASS,
                    buffer.as_mut_ptr() as _,
                    buffer.len() as _,
                    &mut returned_length,
                )
            };

            if nt_success(status) {
                break;
            } else if status == STATUS_INFO_LENGTH_MISMATCH {
                returned_length *= 2;

                let resize_ammount = if buffer.len() > (returned_length as usize) {
                    continue;
                } else {
                    (returned_length as usize) - buffer.len()
                };

                buffer.try_resize(resize_ammount, 0)?;
            } else {
                maple::error!(
                    "zw_query_system_information failed with status: {:x}",
                    status
                );
                return Err(anyhow::Error::msg("zw_query_system_information failed"));
            }
        }

        if returned_length as usize > buffer.len() {
            maple::error!("Buffer still too small after resize");
            return Err(anyhow::Error::msg(
                "Buffer too small for process information",
            ));
        }

        struct ProcessInfoIterator<'a> {
            info: Option<&'a SYSTEM_PROCESS_INFORMATION>,
        }

        impl<'a> Iterator for ProcessInfoIterator<'a> {
            type Item = &'a SYSTEM_PROCESS_INFORMATION;

            fn next(&mut self) -> Option<Self::Item> {
                let current = self.info.take()?;

                self.info = if current.NextEntryOffset == 0 {
                    None
                } else {
                    unsafe {
                        let next = (current as *const _ as *const u8)
                            .add(current.NextEntryOffset as usize)
                            as *const SYSTEM_PROCESS_INFORMATION;
                        Some(&*next)
                    }
                };

                Some(current)
            }
        }

        if returned_length < core::mem::size_of::<SYSTEM_PROCESS_INFORMATION>() as u32 {
            maple::error!("No process information returned");
            return Err(anyhow::Error::msg("No process information available"));
        }

        let mut iter = ProcessInfoIterator {
            info: Some(unsafe { &*(buffer.as_ptr() as *const SYSTEM_PROCESS_INFORMATION) }),
        };

        for proc in iter {
            let pid = proc.UniqueProcessId as u64;
            let eprocess = ps_lookup_by_process_id(pid as _);

            if let Some(eprocess) = eprocess {
                self.container.map_pid(
                    pid,
                    ProcessInformationFactory::try_create_from_eprocess(&eprocess, pid),
                );
            }
        }

        Ok(())
    }
}

struct CacheNotifierCallback {}

impl TaggedObject for CacheNotifierCallback {}

impl PsCreateNotifyCallback for CacheNotifierCallback {
    fn on_create(
        &self,
        process: wdrf_std::object::ArcKernelObj<wdrf_std::structs::PKPROCESS>,
        pid: windows_sys::Win32::Foundation::HANDLE,
        create_info: &wdrf::process::PsCreateNotifyInfo,
    ) -> wdrf_std::NtResult<()> {
        let pid = pid as u64;
        let cache = &DRIVER_CONTEXT.get().process_cache;

        let uid = cache.internal_on_process_create(&process, pid, create_info);

        let process_info = cache.get_process_info_from_uid(uid);

        if let Some(process_info) = process_info {
            let op: EventProcessOperation = EventProcessOperation::ProcessCreate {
                pid,
                cmd: process_info.cmd,
            };
            let event = KmMessage {
                event: EventCompoent {
                    date: SystemTime::new().raw_time(),
                    thread: create_info.client_id.UniqueThread as _,
                    operation: EventClass::Process(op),
                    result: STATUS_SUCCESS,
                    path: process_info.path,
                    duration: 0,
                },
                process: SimpleProcessDetails {
                    pid,
                    unique_id: uid,
                },
                stack: EventStack::new(),
            };

            let _ = DRIVER_CONTEXT.get().communication.try_send_event(event);
        }

        Ok(())
    }

    fn on_destroy(&self, pid: windows_sys::Win32::Foundation::HANDLE) {
        let pid = pid as u64;
        let cache = &DRIVER_CONTEXT.get().process_cache;

        let unique_id = DRIVER_CONTEXT
            .get()
            .process_cache
            .internal_on_process_exit(pid);

        if let Some(uid) = unique_id {
            let process_info = cache.get_process_info_from_uid(uid);

            if let Some(process_info) = process_info {
                let op: EventProcessOperation = EventProcessOperation::ProcessDestroy { pid };
                let event = KmMessage {
                    event: EventCompoent {
                        date: SystemTime::new().raw_time(),
                        thread: unsafe { PsGetCurrentThreadId() as _ },
                        operation: EventClass::Process(op),
                        result: STATUS_SUCCESS,
                        path: SerializableNtString::empty(),
                        duration: 0,
                    },
                    process: SimpleProcessDetails {
                        pid,
                        unique_id: uid,
                    },
                    stack: EventStack::new(),
                };

                let _ = DRIVER_CONTEXT.get().communication.try_send_event(event);
            }
        }
    }
}
