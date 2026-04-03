#![no_std]
#![no_main]

///rustup target add thumbv7em-none-eabihf
/// cargo build --target thumbv7em-none-eabihf
use core::panic::PanicInfo;

static HELLO: &[u8] = b"Hello World!";

/// 这个函数将作为程序的入口点,并且永远不会返回，禁用名称重整
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}

/// 这个函数将在 panic 时被调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
