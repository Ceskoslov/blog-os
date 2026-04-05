//! 内存分配器模块
//!
//! 本模块实现了堆内存的初始化和管理，包括：
//! - 堆内存的映射和初始化
//! - 全局内存分配器的设置
//! - 各种分配策略的实现（块分配器等）

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub mod bump;
pub mod linked_list;
pub mod fixed_size_block;

/// 虚拟分配器，不实现真正的分配功能（仅用于测试）
pub struct Dummy;

/// 堆内存的虚拟起始地址
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// 堆内存的大小：100KB
pub const HEAP_SIZE: usize = 100 * 1024;

/// 初始化堆内存
///
/// 为堆分配物理内存框架，并将其映射到虚拟地址空间中。
/// 然后初始化全局分配器以使用这些映射的内存。
///
/// # 参数
///
/// * `mapper` - 虚拟内存映射器
/// * `frame_allocator` - 物理内存框架分配器
///
/// # 返回值
///
/// 成功则返回 Ok(())，否则返回 MapToError
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // 计算堆所占用的页面范围
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // 为每一页分配物理框架并建立虚拟内存映射
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    // 初始化全局分配器
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should not be called")
    }
}

use fixed_size_block::FixedSizeBlockAllocator;

/// 全局内存分配器实例
/// 使用固定大小块分配器作为具体实现
#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(
    FixedSizeBlockAllocator::new());

/// 为任意类型提供线程安全的互斥锁包装
///
/// 使用 spin::Mutex 提供自旋锁实现，避免依赖操作系统线程原语
pub struct Locked<A> {
    /// 被保护的资源
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// 创建一个新的 Locked 包装器
    ///
    /// # 参数
    ///
    /// * `inner` - 要被保护的资源
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// 获取对内部资源的锁定访问
    ///
    /// # 返回值
    ///
    /// 返回一个 MutexGuard，用于访问被保护的资源
    pub fn lock(&self) -> spin::MutexGuard<'_, A> {
        self.inner.lock()
    }
}

/// 将地址向上对齐到指定的对齐边界
///
/// 这个函数确保内存地址遵守对齐要求。
///
/// # 参数
///
/// * `addr` - 需要对齐的地址
/// * `align` - 对齐粒度（必须是2的幂）
///
/// # 返回值
///
/// 返回大于等于 `addr` 的最小的能被 `align` 整除的地址
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
