use maple::{error, info};
use nt_string::unicode_string::NtUnicodeStr;
use wdrf::minifilter::{
    communication::client_communication::{FltClientCommunication, FltCommunicationCallback},
    FltFilter,
};
use wdrf_std::time::Timeout;
use wdrf_std::NtResult;

struct CommunncationCallback {}

unsafe impl Send for CommunncationCallback {}
unsafe impl Sync for CommunncationCallback {}

impl FltCommunicationCallback for CommunncationCallback {
    fn connect(&self, _buffer: Option<&[u8]>) -> anyhow::Result<()> {
        info!("Client connected");

        Ok(())
    }

    fn message(
        &self,
        _input: &[u8],
        _output: Option<&mut wdrf_std::slice::tracked_slice::TrackedSlice>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn disconnect(&self) {
        info!("Client disconnected");
    }
}

pub struct PortCommunication {
    flt_communication: FltClientCommunication<CommunncationCallback>,
}

impl PortCommunication {
    pub fn try_create(filter: FltFilter, name: NtUnicodeStr) -> anyhow::Result<Self> {
        let communication = FltClientCommunication::new(CommunncationCallback {}, filter, name);
        if let Err(status) = communication {
            error!("FltClientCommunication failed with status: {status}");
            anyhow::bail!("Failed to create flt communication");
        }
        let communication = communication.unwrap();

        Ok(Self {
            flt_communication: communication,
        })
    }

    #[inline]
    pub fn send_with_reply<'a>(
        &self,
        input: &[u8],
        output: &'a mut [u8],
        timeout: Timeout,
    ) -> NtResult<&'a [u8]> {
        self.flt_communication
            .send_message_with_reply(input, output, timeout)
    }
}
