use core::time::Duration;

use kmum_common::{serializable_ntstring::SerializableNtString, KmMessage};
use nt_string::unicode_string::NtUnicodeString;
use wdrf::minifilter::filter::{FileNameInformation, FltPreOpCallback, PreOpStatus};
use wdrf_std::{kmalloc::TaggedObject, time::Timeout};

use crate::global::DRIVER_CONTEXT;

pub struct ProcmonMinifilterPreOp {}

impl FltPreOpCallback for ProcmonMinifilterPreOp {
    fn callback<'a>(
        &self,
        data: wdrf::minifilter::filter::FltCallbackData<'a>,
        related_obj: wdrf::minifilter::filter::FltRelatedObjects<'a>,
        params: wdrf::minifilter::filter::params::FltParameters<'a>,
    ) -> wdrf::minifilter::filter::PreOpStatus {
        let communication = &DRIVER_CONTEXT.get().communication;

        let name = FileNameInformation::create(&data);

        if let Ok(name) = name {
            let name = name.name();
            match NtUnicodeString::try_from(name.as_u16str()) {
                Ok(name) => {
                    let _ = communication.send_no_reply(
                        &KmMessage::CreateFile(SerializableNtString::from(name)),
                        Timeout::from_duration(Duration::from_secs(1)),
                    );
                }
                _ => {}
            };
        }

        PreOpStatus::SuccessNoCallback
    }
}

impl TaggedObject for ProcmonMinifilterPreOp {}
