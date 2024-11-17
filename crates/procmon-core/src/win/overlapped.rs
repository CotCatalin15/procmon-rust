use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_IO_INCOMPLETE, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::WaitForSingleObject;
use windows_sys::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};

use super::{event::Event, CloseHandlePolicy, Handle};

#[repr(transparent)]
pub struct Overlapped(OVERLAPPED);

impl Overlapped {
    pub fn new() -> Option<Self> {
        let event = Event::new();

        if let Some(event) = event {
            let ov = unsafe {
                OVERLAPPED {
                    hEvent: Event::leak_handle(event),
                    ..core::mem::zeroed()
                }
            };
            Some(Self(ov))
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        let event = self.0.hEvent;
        self.0 = unsafe { core::mem::zeroed() };
        self.0.hEvent = event;
    }

    pub fn ov(&self) -> &OVERLAPPED {
        &self.0
    }

    pub fn mut_ov(&mut self) -> &mut OVERLAPPED {
        &mut self.0
    }
}

impl Drop for Overlapped {
    fn drop(&mut self) {
        unsafe {
            Handle::<CloseHandlePolicy>::new_unchecked(self.0.hEvent);
        }
    }
}

pub struct OverlappedJoin<'a> {
    handle: HANDLE,
    overlapped: &'a mut Overlapped,
}

impl<'a> OverlappedJoin<'a> {
    pub fn new(ov: &'a mut Overlapped, handle: HANDLE) -> Self {
        Self {
            overlapped: ov,
            handle,
        }
    }

    pub fn event_handle(&self) -> HANDLE {
        self.overlapped.0.hEvent
    }

    pub fn get_result(&self, wait: bool) -> Option<u32> {
        let mut bytes_transfered = 0;
        let status = unsafe {
            GetOverlappedResult(
                self.handle,
                self.overlapped.ov(),
                &mut bytes_transfered,
                wait as _,
            ) == 1
        };
        if status {
            Some(bytes_transfered)
        } else {
            let error = unsafe { GetLastError() };

            if error != ERROR_IO_INCOMPLETE {
                panic!("GetOverlappedResult failed with status: 0x{:x}", error);
            }

            None
        }
    }

    pub fn join(self) -> u32 {
        self.get_result(true).unwrap()
    }

    pub fn cacel(self) -> bool {
        unsafe { CancelIo(self.handle) == 1 }
    }

    pub fn is_ready(&self) -> bool {
        let status = unsafe { WaitForSingleObject(self.overlapped.ov().hEvent, 0) };
        match status {
            WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            _ => panic!("WaitForSingleObject failed with status: 0x{:x}", status),
        }
    }
}

impl<'a> Drop for OverlappedJoin<'a> {
    fn drop(&mut self) {
        unsafe {
            CancelIo(self.handle);
        }
    }
}
