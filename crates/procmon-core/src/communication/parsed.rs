use std::alloc::Layout;

use windows_sys::Win32::{
    Foundation::NTSTATUS,
    Storage::InstallableFileSystems::{FILTER_MESSAGE_HEADER, FILTER_REPLY_HEADER},
};

pub struct FilterMessageBuffer {
    buffer: Vec<u8>,
}

pub struct FilterReplyBuffer {
    buffer: Vec<u8>,
}

impl FilterMessageBuffer {
    pub fn new(buffer_size: usize) -> Self {
        let total_size = buffer_size + std::mem::size_of::<FILTER_MESSAGE_HEADER>();
        let layout = Layout::from_size_align(total_size, 8).unwrap(); // 8-byte alignment
        let ptr = unsafe { std::alloc::alloc(layout) };
        let buffer = unsafe { Vec::from_raw_parts(ptr, total_size, total_size) };

        Self { buffer }
    }

    pub fn mut_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn as_parsed(&self, msg_size: usize) -> ParsedMessage<'_> {
        ParsedMessage::new(&self.buffer[..msg_size])
    }
}

impl FilterReplyBuffer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: vec![0; buffer_size],
        }
    }

    pub fn as_parsed(&mut self) -> ParsedReply<'_> {
        ParsedReply::new(&mut self.buffer)
    }

    pub fn as_buffer(&self) -> &[u8] {
        &self.buffer
    }
}

pub struct ParsedMessage<'a> {
    pub header: &'a FILTER_MESSAGE_HEADER,
    pub buffer: &'a [u8],
}

pub struct ParsedReply<'a> {
    pub header: &'a mut FILTER_REPLY_HEADER,
    pub buffer: &'a mut [u8],
}

impl<'a> ParsedMessage<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        // Read header from the start of the buffer
        let header: &'a FILTER_MESSAGE_HEADER =
            unsafe { &*(buffer.as_ptr() as *const FILTER_MESSAGE_HEADER) };
        // Data starts after the header
        let data_start = core::mem::size_of::<FILTER_MESSAGE_HEADER>();
        let data_end = buffer.len();
        let buffer_data = &buffer[data_start..data_end];

        Self {
            header,
            buffer: buffer_data,
        }
    }
}

impl<'a> ParsedReply<'a> {
    fn new(buffer: &'a mut [u8]) -> Self {
        let header: &'a mut FILTER_REPLY_HEADER = unsafe {
            let ptr = buffer.as_ptr() as *mut FILTER_REPLY_HEADER;

            &mut *ptr
        };

        Self {
            header: header,
            buffer: &mut buffer[std::mem::size_of::<FILTER_REPLY_HEADER>()..],
        }
    }

    pub fn construct_reply(&mut self, message: &ParsedMessage, status: NTSTATUS) {
        self.header.Status = status;
        self.header.MessageId = message.header.MessageId;
    }
}
