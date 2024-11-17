use windows_sys::Wdk::System::SystemServices::KeBugCheck;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        KeBugCheck(0x1234);
    }
    loop {}
}
