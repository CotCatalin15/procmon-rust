#![allow(dead_code)]

pub mod constatns;
pub mod event;
pub mod overlapped;

use std::{marker::PhantomData, mem::forget};
use windows_sys::{
    core::HRESULT,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Diagnostics::Debug::FACILITY_WIN32,
    },
};

pub trait HandlePolicy {
    fn is_valid(handle: HANDLE) -> bool;
    fn close_handle(handle: HANDLE);
}

pub struct CloseHandlePolicy;
impl HandlePolicy for CloseHandlePolicy {
    fn is_valid(handle: HANDLE) -> bool {
        handle != 0
    }

    fn close_handle(handle: HANDLE) {
        unsafe {
            CloseHandle(handle);
        }
    }
}

#[repr(transparent)]
pub struct Handle<P: HandlePolicy = CloseHandlePolicy> {
    inner: HANDLE,
    _phantom: PhantomData<P>,
}

impl<P: HandlePolicy> Handle<P> {
    pub fn new(handle: HANDLE) -> Option<Self> {
        if P::is_valid(handle) {
            Some(Self {
                inner: handle,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }

    ///
    /// SAFETY:
    ///
    /// Ensure the handle is valid
    ///
    pub unsafe fn new_unchecked(handle: HANDLE) -> Self {
        Self {
            inner: handle,
            _phantom: PhantomData,
        }
    }

    pub fn leak(self) -> HANDLE {
        let handle = self.inner;
        forget(self);
        handle
    }

    pub fn handle(&self) -> HANDLE {
        self.inner
    }

    pub fn put(&mut self) -> &mut HANDLE {
        &mut self.inner
    }
}

impl<P: HandlePolicy> Drop for Handle<P> {
    fn drop(&mut self) {
        P::close_handle(self.inner);
    }
}

pub fn hresult_from_win32(error: i32) -> HRESULT {
    if error <= 0 {
        error
    } else {
        (error & 0x0000FFFF) | ((FACILITY_WIN32 << 16) as i32) | (0x80000000u32 as i32)
    }
}
