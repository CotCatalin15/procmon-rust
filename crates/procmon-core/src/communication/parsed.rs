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
        Self {
            buffer: vec![0; buffer_size],
        }
    }

    pub fn mut_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn as_parsed(&self) -> ParsedMessage<'_> {
        ParsedMessage::new(&self.buffer)
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
        let header: &'a FILTER_MESSAGE_HEADER = unsafe {
            let ptr = buffer.as_ptr() as *const FILTER_MESSAGE_HEADER;
            &*ptr
        };

        Self {
            header: header,
            buffer: &buffer[std::mem::size_of::<FILTER_MESSAGE_HEADER>()..],
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
