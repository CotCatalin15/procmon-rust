// src/lib.rs

use std::{
    io::Write,
    sync::{Arc, Mutex},
};
use uniffi::*;

#[derive(Debug)]
pub struct ProcmonCore {
    memory_database_name: String,
    rutime: tokio::runtime::Handle,
}

impl ProcmonCore {
    pub fn new(memory_database_name: String) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        ProcmonCore {
            memory_database_name,
            runtime: 
        }
    }

    pub fn start(&self) {
        std::fs::File::create("test.txt")
            .unwrap()
            .write_all(self.memory_database_name.as_bytes())
            .unwrap();
    }

    pub fn stop(&self) {}
}

// Include the UniFFI scaffolding
uniffi::include_scaffolding!("procmon");
