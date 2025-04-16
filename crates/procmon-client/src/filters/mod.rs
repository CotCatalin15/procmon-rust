use kmum_common::KmMessage;

#[derive(Clone, Copy)]
pub enum SimpleFilter {
    FilterPid(u64),
    FilterPidLessEq(u64),
}

impl SimpleFilter {
    pub fn matches(&self, event: &KmMessage) -> bool {
        match self {
            SimpleFilter::FilterPid(pid) => event.process.pid == *pid,
            SimpleFilter::FilterPidLessEq(pid) => event.process.pid <= *pid,
        }
    }
}
