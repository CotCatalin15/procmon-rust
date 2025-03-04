use wdrf::minifilter::filter::{FileNameInformation, FltPreOpCallback, PreOpStatus};
use wdrf_std::kmalloc::TaggedObject;

use crate::global::DRIVER_CONTEXT;

pub struct ProcmonMinifilterPreOp {}

impl FltPreOpCallback for ProcmonMinifilterPreOp {
    fn callback<'a>(
        &self,
        data: wdrf::minifilter::filter::FltCallbackData<'a>,
        _related_obj: wdrf::minifilter::filter::FltRelatedObjects<'a>,
        _params: wdrf::minifilter::filter::params::FltParameters<'a>,
    ) -> wdrf::minifilter::filter::PreOpStatus {
        let communication = &DRIVER_CONTEXT.get().communication;

        let name = FileNameInformation::create(&data);

        PreOpStatus::SuccessNoCallback
    }
}

impl TaggedObject for ProcmonMinifilterPreOp {}
