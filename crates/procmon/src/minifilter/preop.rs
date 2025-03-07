use kmum_common::{
    event::{
        EventClass, EventCompoent, EventFileSystemOperation, EventStack, SimpleProcessDetails,
    },
    serializable_ntstring::SerializableNtString,
    KmMessage,
};
use nt_string::unicode_string::NtUnicodeString;
use wdrf::minifilter::filter::{
    params::FltParameters, FileNameInformation, FltPostOpCallback, FltPreOpCallback, PostOpContext,
    PostOpStatus, PreOpStatus,
};
use wdrf_std::{kmalloc::TaggedObject, time::SystemTime};
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

pub struct PostCallbackContext {
    pre_time: SystemTime,
    path: NtUnicodeString,
    uid: u64,
}

unsafe impl Send for PostCallbackContext {}
impl TaggedObject for PostCallbackContext {}

impl<'a> FltPreOpCallback<'a> for ProcmonMinifilterCallback {
    type MinifilterContext = ProcmonMinifilterContext;
    type PostContext = PostCallbackContext;

    fn call_pre(
        minifilter_context: &'a Self::MinifilterContext,
        data: wdrf::minifilter::filter::FltCallbackData<'a>,
        related_obj: wdrf::minifilter::filter::FltRelatedObjects<'a>,
        params: wdrf::minifilter::filter::params::FltParameters<'a>,
    ) -> PreOpStatus<Self::PostContext> {
        let name = FileNameInformation::create(&data).ok();

        let path = name
            .as_ref()
            .map(|info| NtUnicodeString::from(&info.name()));

        let pid = unsafe { FltGetRequestorProcessId(data.raw_struct()) } as u64;
        let uid = DRIVER_CONTEXT.get().process_cache.pid_to_unique_id(pid);

        if let Some(path) = path
            && let Some(uid) = uid
            && let Ok(context) = PostOpContext::try_create(PostCallbackContext {
                uid,
                pre_time: SystemTime::new(),
                path,
            })
        {
            PreOpStatus::SuccessWithCallback(Some(context))
        } else {
            PreOpStatus::SuccessNoCallback
        }
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

        let PostCallbackContext {
            uid: uid,
            pre_time: preop_time,
            path: path,
        } = context.unwrap().unwrap();

        let pid = unsafe { FltGetRequestorProcessId(data.raw_struct()) } as u64;
        let op = Self::map_minifilter_param_to_event_op(&params);

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

        PostOpStatus::FinishProcessing
    }
}

impl ProcmonMinifilterCallback {
    fn map_minifilter_param_to_event_op(param: &FltParameters) -> EventFileSystemOperation {
        match param {
            FltParameters::Create(flt_create_request) => EventFileSystemOperation::Create {
                attribute: flt_create_request.attributes(),
            },
            FltParameters::Read(flt_read_file_request) => EventFileSystemOperation::Read {
                length: flt_read_file_request.len() as _,
                offset: flt_read_file_request.offset(),
            },
            FltParameters::Write(flt_write_file_request) => EventFileSystemOperation::Write {
                length: flt_write_file_request.len() as _,
                offset: flt_write_file_request.offset(),
            },
            FltParameters::Close(flt_close_file_request) => EventFileSystemOperation::Close {},
        }
    }
}
