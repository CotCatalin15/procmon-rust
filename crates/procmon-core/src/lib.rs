#![allow(internal_features)]
#![feature(core_intrinsics)]

use std::hint::black_box;

use communication::Communication;
use kmum_common::UmSendMessage;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

use tracing::info;

pub mod communication;
pub mod win;

pub fn test() {
    let communication = Communication::new();
    black_box(&communication);

    let _pid = unsafe { GetCurrentProcessId() as u64 };

    loop {
        let mut input = String::new();

        println!("Enter an pid:");

        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let number: i32 = match input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Please enter a valid integer.");
                continue;
            }
        };

        println!("You entered: {}", number);

        let result = communication.send_message(UmSendMessage::GetPidInfo(number as _));

        if let Ok(reply) = result {
            if let Some(reply) = reply {
                info!("Received reply for current pid: {:#?}", reply);
            }
        }
    }
}
