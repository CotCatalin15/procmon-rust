use kmum_common::{
    event::{
        EventClass, EventCompoent, EventFileSystemOperation, EventStack, SimpleProcessDetails,
    },
    serializable_ntstring::SerializableNtString,
    KmMessage,
};
use nt_string::unicode_string::NtUnicodeString;
use wdrf::minifilter::filter::{
    FileNameInformation, FltPostOpCallback, FltPreOpCallback, PostOpContext, PostOpStatus,
    PreOpStatus,
};
use wdrf_std::{kmalloc::TaggedObject, time::SystemTime, traits::DispatchSafe};
use windows_sys::{
    Wdk::{
        Storage::FileSystem::Minifilters::FltGetRequestorProcessId,
        System::SystemServices::PsGetCurrentThreadId,
    },
    Win32::Foundation::STATUS_SUCCESS,
};

use crate::global::DRIVER_CONTEXT;

pub struct ProcmonMinifilterCallback;

pub struct ProcmonMinifilterContext;

impl TaggedObject for ProcmonMinifilterContext {}
impl TaggedObject for ProcmonMinifilterCallback {}

impl<'a> FltPreOpCallback<'a> for ProcmonMinifilterCallback {
    type MinifilterContext = ProcmonMinifilterContext;
    type PostContext = SystemTime;

    fn call_pre(
        minifilter_context: &'a Self::MinifilterContext,
        data: wdrf::minifilter::filter::FltCallbackData<'a>,
        related_obj: wdrf::minifilter::filter::FltRelatedObjects<'a>,
        params: wdrf::minifilter::filter::params::FltParameters<'a>,
    ) -> PreOpStatus<Self::PostContext> {
        PreOpStatus::SuccessWithCallback(PostOpContext::try_create(SystemTime::new()).ok())
    }
}

impl<'a> FltPostOpCallback<'a> for ProcmonMinifilterCallback {
    fn call_post(
        minifilter_context: &'static Self::MinifilterContext,
        data: wdrf::minifilter::filter::FltCallbackData<'a>,
        related_obj: wdrf::minifilter::filter::FltRelatedObjects<'a>,
        params: wdrf::minifilter::filter::params::FltParameters<'a>,
        context: Option<PostOpContext<Self::PostContext>>,
        draining: bool,
    ) -> PostOpStatus {
        let communication = &DRIVER_CONTEXT.get().communication;

        let preop_time = if let Some(context) = context {
            *context
        } else {
            SystemTime::new()
        };

        let name = FileNameInformation::create(&data).ok();
        let pid = unsafe { FltGetRequestorProcessId(data.raw_struct()) } as u64;
        let uid = DRIVER_CONTEXT.get().process_cache.pid_to_unique_id(pid);
        let path = name
            .as_ref()
            .map(|info| NtUnicodeString::from(&info.name()));

        if let Some(name) = name
            && let Some(uid) = uid
            && let Some(path) = path
        {
            let op = EventFileSystemOperation::Create { attribute: 0 };

            let event = KmMessage {
                event: EventCompoent {
                    date: SystemTime::new().raw_time(),
                    thread: unsafe { PsGetCurrentThreadId() as _ },
                    operation: EventClass::FileSystem(op),
                    result: STATUS_SUCCESS,
                    path: SerializableNtString::new(path),
                    duration: SystemTime::new().raw_time() - preop_time.raw_time(),
                },
                process: SimpleProcessDetails {
                    pid,
                    unique_id: uid,
                },
                stack: EventStack::new(),
            };

            let _ = communication.try_send_event(event);
        }

        PostOpStatus::FinishProcessing
    }
}
