#[derive(Debug)]
pub struct Event {
    pub id: u64,
    pub timestamp: u64,
    pub name: String,
}

pub struct EventReader {}

impl EventReader {
    pub fn get(&self, index: usize) -> Option<Event> {
        Some(Event {
            id: index as u64,
            timestamp: 1234,
            name: index.to_string(),
        })
    }
}
