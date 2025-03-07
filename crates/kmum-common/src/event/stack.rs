use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EventStack {}

impl EventStack {
    pub fn new() -> Self {
        Self {}
    }
}
