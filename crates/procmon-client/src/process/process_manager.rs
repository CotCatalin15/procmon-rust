use kmum_common::{KmReplyMessage, UmSendMessage};

pub struct ProcessManager {}

impl ProcessManager {
    pub fn new<Q>(query_cb: Q) -> Self
    where
        Q: Fn(UmSendMessage) -> Option<KmReplyMessage>,
    {
        Self {}
    }
}
