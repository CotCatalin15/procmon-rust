#![allow(internal_features)]
#![feature(core_intrinsics)]

use std::{hint::black_box, intrinsics::breakpoint, time::Duration};

use communication::Communication;

pub mod communication;
pub mod win;

pub fn test() {
    unsafe {
        breakpoint();
    }

    let communication = Communication::new();
    black_box(&communication);

    std::thread::sleep(Duration::MAX);
}
