//! 中断处理模块
//!
//! 本模块负责设置和管理x86_64架构的中断处理。包括：
//! - 中断描述符表(IDT)的初始化
//! - 各类中断处理器的定义（定时器、键盘、页面错误等）
//! - 可编程中断控制器(PIC)的配置

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::println;
use lazy_static::lazy_static;
use crate::gdt;
use pic8259::ChainedPics;
use spin;
use crate::print;
use crate::hlt_loop;

/// PIC 1的中断偏移量，范围 32-39
pub const PIC_1_OFFSET: u8 = 32;
/// PIC 2的中断偏移量，范围 40-47
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// 全局可编程中断控制器实例
/// ChainedPics用于处理x86架构中的两个8259A芯片（主从级联）
pub static PIC: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe {
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
});

/// 中断索引枚举，定义各类硬件中断
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    /// 定时器中断（IRQ 0）
    Timer = PIC_1_OFFSET,
    /// 键盘中断（IRQ 1）
    Keyboard,
}

impl InterruptIndex {
    /// 将中断索引转换为u8
    fn as_u8(self) -> u8 {
        self as u8
    }

    /// 将中断索引转换为usize，用于IDT数组索引
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    /// 全局中断描述符表
    /// 使用lazy_static保证只初始化一次，且在第一次访问时初始化
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // 设置异常处理器
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);

        // 设置硬件中断处理器
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// 加载IDT到处理器
pub fn init_idt() {
    IDT.load();
}

/// 断点异常处理器
/// 当执行int3指令时触发（通常用于调试）
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint hit at {:?}", stack_frame);
}

/// 双重故障异常处理器
/// 当CPU在处理另一个异常时再次触发异常时调用
/// 这是一个致命错误，无法恢复
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: Double fault at {:?}", stack_frame);
}

/// 页面错误异常处理器
/// 当CPU尝试访问未映射的虚拟内存时触发
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Addr: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("Stack Frame: {:?}", stack_frame);
    hlt_loop();
}

/// 定时器中断处理器
/// 每当定时器芯片产生中断时调用（通常是固定频率）
/// 打印"."用于可视化中断频率
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe {
        PIC.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// 键盘中断处理器
/// 当键盘产生扫描码时触发
/// 将扫描码传递给任务模块进行处理
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    // 从键盘端口读取扫描码
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // 将扫描码添加到键盘任务队列
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PIC.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

/// 测试用例：测试断点异常
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
