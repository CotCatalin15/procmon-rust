use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{CreateEventW, ResetEvent, SetEvent, WaitForSingleObject, INFINITE},
};

use super::Handle;

pub struct Event(Handle);

impl Event {
    pub fn new() -> Option<Self> {
        let event =
            unsafe { CreateEventW(core::ptr::null(), true as _, false as _, core::ptr::null()) };

        Handle::new(event).map(Self)
    }

    pub fn leak_handle(self) -> HANDLE {
        self.0.leak()
    }

    #[inline]
    pub fn signal(&self) {
        unsafe {
            SetEvent(self.handle());
        }
    }

    #[inline]
    pub fn clear(&self) {
        unsafe {
            ResetEvent(self.handle());
        }
    }

    pub fn wait(&self) -> bool {
        let status = unsafe { WaitForSingleObject(self.handle(), INFINITE) };

        match status {
            WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            _ => panic!("WaitForSingleObject failed with status: 0x{:x}", status),
        }
    }

    #[inline]
    pub fn handle(&self) -> HANDLE {
        self.0.handle()
    }
}
