use kmum_common::{
    krnmsg::{KmMessageCommonHeader, KmMessageEventKind, ProcessCreateEvent, ProcessDestroyEvent},
    process::{ProcessInformation, UniqueProcessId},
    KmMessage,
};
use wdrf::process::{
    notifier::PsNotifierRegistration, process_create_notifier::PsCreateNotifyCallback,
    PsCreateNotifyInfo,
};
use wdrf_std::{kmalloc::TaggedObject, object::ArcKernelObj, structs::PKPROCESS, time::SystemTime};
use windows_sys::Win32::Foundation::STATUS_SUCCESS;

use crate::global::DRIVER_CONTEXT;

use super::collection::PsInfoContainer;

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
            .map_err(|_| anyhow::Error::msg("Failed to start process collector cache"))
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
        self.container.get_process_info_from_uid(unique_id)
    }

    pub fn pid_to_unique_id(&self, pid: u64) -> Option<UniqueProcessId> {
        self.container.pid_to_unique_id(pid)
    }

    fn internal_on_process_create(
        &self,
        eprocess: &ArcKernelObj<PKPROCESS>,
        pid: u64,
        process_info: &PsCreateNotifyInfo,
    ) -> Option<UniqueProcessId> {
        self.container
            .register_from_process_create(eprocess, pid as _, process_info)
    }

    fn internal_on_process_exit(&self, pid: u64) -> Option<UniqueProcessId> {
        self.container.register_from_process_destroy(pid)
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

        let unique_id = cache.internal_on_process_create(&process, pid, create_info);

        if let Some(uid) = unique_id {
            let process_info = cache.get_process_info_from_uid(uid);

            if let Some(process_info) = process_info {
                let event = KmMessage {
                    common: KmMessageCommonHeader {
                        operation: kmum_common::krnmsg::KmMessageOperationType::ProcessCreate,
                        timestamp: SystemTime::new().raw_time(),
                        pid: pid as _,
                        thread_id: create_info.client_id.UniqueThread as _,
                        class: 0,
                        result: STATUS_SUCCESS,
                        path: process_info.path,
                        duration: 0,
                        unique_pid: uid,
                    },
                    event: KmMessageEventKind::ProcessCreate(ProcessCreateEvent {
                        pid: pid,
                        cmd: process_info.cmd,
                    }),
                };

                let _ = DRIVER_CONTEXT.get().communication.try_send_event(event);
            }
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
                let event = KmMessage {
                    common: KmMessageCommonHeader {
                        operation: kmum_common::krnmsg::KmMessageOperationType::ProcessDestroy,
                        timestamp: SystemTime::new().raw_time(),
                        pid: pid as _,
                        thread_id: 0,
                        class: 0,
                        result: STATUS_SUCCESS,
                        path: process_info.path,
                        duration: 0,
                        unique_pid: uid,
                    },
                    event: KmMessageEventKind::ProcessDestroy(ProcessDestroyEvent { pid: pid }),
                };

                let _ = DRIVER_CONTEXT.get().communication.try_send_event(event);
            }
        }
    }
}
