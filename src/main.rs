#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use blog_os::println;
use bootloader::{BootInfo, entry_point};
use blog_os::task::{Task, executor::Executor};
use blog_os::task::keyboard;
use blog_os::allocator;

entry_point!(kernel_main);

/// 异步任务：返回一个测试数字
async fn async_number() -> u32 {
    42
}

/// 示例异步任务，演示异步/await的使用
async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

/// 内核入口点
///
/// 初始化所有系统组件，包括：
/// - 内存管理和分页
/// - 堆分配器
/// - 中断处理
/// - 异步任务执行器
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::memory;
    use x86_64::VirtAddr;
    use blog_os::memory::BootInfoFrameAllocator;

    println!("Hello World{}", "!");
    blog_os::init();

    // 初始化虚拟内存映射
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // 初始化堆内存分配器
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    // 创建异步执行器并运行任务
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

/// 内核panic处理器
///
/// 当程序遇到致命错误时调用，打印panic信息并进入无限循环
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}\n", info);
    blog_os::hlt_loop();
}

/// 测试时的panic处理器
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}
