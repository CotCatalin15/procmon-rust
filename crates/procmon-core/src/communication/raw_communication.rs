use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_FLT_NO_WAITER_FOR_REPLY, ERROR_IO_PENDING, HANDLE, NO_ERROR, S_OK,
    },
    Storage::InstallableFileSystems::{
        FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage, FilterSendMessage,
    },
    System::IO::OVERLAPPED,
};

use crate::win::hresult_from_win32;

use super::CommunicationError;

struct PortHandle {
    handle: HANDLE,
}

pub struct RawCommunication {
    port: PortHandle,
}

unsafe impl Send for RawCommunication {}
unsafe impl Sync for RawCommunication {}

impl PortHandle {
    pub fn new(name: &[u16]) -> anyhow::Result<Self> {
        let mut handle = 0;
        let status = unsafe {
            FilterConnectCommunicationPort(
                name.as_ptr(),
                0,
                core::ptr::null(),
                0,
                core::ptr::null(),
                &mut handle,
            )
        };
        anyhow::ensure!(
            status == NO_ERROR as _,
            "FilterConnectCommunicationPort failed"
        );
        Ok(Self { handle })
    }
}

impl Drop for PortHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

impl RawCommunication {
    pub fn new(name: &[u16]) -> anyhow::Result<Self> {
        let port = PortHandle::new(name)?;

        Ok(Self { port })
    }

    pub fn handle(&self) -> HANDLE {
        self.port.handle
    }

    pub fn send_buffer(
        &self,
        buffer: &[u8],
        output: Option<&mut [u8]>,
    ) -> anyhow::Result<u32, CommunicationError> {
        let mut bytes_written = 0;

        let (out_buffer, out_buffer_len) = if let Some(output) = output {
            (output.as_mut_ptr(), output.len())
        } else {
            (core::ptr::null_mut(), 0)
        };

        let status = unsafe {
            FilterSendMessage(
                self.port.handle,
                buffer.as_ptr() as _,
                buffer.len() as _,
                out_buffer as _,
                out_buffer_len as _,
                &mut bytes_written,
            )
        };
        if status == S_OK as _ {
            Ok(bytes_written)
        } else {
            Err(CommunicationError::Port)
        }
    }

    #[allow(dead_code)]
    ///
    /// # Safety
    ///
    /// buffer must start with FILTER_REPLY_HEADER
    ///
    pub unsafe fn get_message_raw(
        &self,
        buffer: &mut [u8],
    ) -> anyhow::Result<(), CommunicationError> {
        let status = FilterGetMessage(
            self.port.handle,
            buffer.as_mut_ptr().cast(),
            buffer.len() as _,
            core::ptr::null_mut(),
        );
        if status == S_OK as _ {
            Ok(())
        } else {
            Err(CommunicationError::Port)
        }
    }

    ///
    /// # Safety
    ///
    /// buffer must start with FILTER_REPLY_HEADER
    ///
    pub unsafe fn get_message_overlapped_raw(
        &self,
        buffer: &mut [u8],
        overlapped: &mut OVERLAPPED,
    ) -> anyhow::Result<(), CommunicationError> {
        let status = FilterGetMessage(
            self.port.handle,
            buffer.as_mut_ptr().cast(),
            buffer.len() as _,
            overlapped,
        );
        if status == hresult_from_win32(ERROR_IO_PENDING as _) {
            Ok(())
        } else {
            Err(CommunicationError::Port)
        }
    }

    ///
    /// # Safety
    ///
    /// buffer must start with FILTER_REPLY_HEADER
    ///
    pub unsafe fn reply_message_raw(
        &self,
        buffer: &[u8],
    ) -> anyhow::Result<(), CommunicationError> {
        let status =
            FilterReplyMessage(self.port.handle, buffer.as_ptr().cast(), buffer.len() as _);
        if status == S_OK as _ {
            Ok(())
        } else if status == ERROR_FLT_NO_WAITER_FOR_REPLY {
            Err(CommunicationError::NoWaiterPresent)
        } else {
            println!("FilterReplyMessage with status: {:x}", status);
            Err(CommunicationError::Port)
        }
    }
}
