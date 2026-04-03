#![no_std]
#![no_main]

///rustup target add thumbv7em-none-eabihf
/// cargo build --target thumbv7em-none-eabihf
use core::panic::PanicInfo;

/// 这个函数将作为程序的入口点,并且永远不会返回，禁用名称重整
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {}
}

/// 这个函数将在 panic 时被调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

