#![no_std]

use core::ptr::NonNull;

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///

/// 以下EarlyAllocator的实现极度简化
/// 首先，EarlyAllocator可以分配字节，也可以分配页，并共用同一块空间来进行分配。
/// 对于字节分配，不去记录已分配的字节区域的开始位置及长度，仅仅记录一个分配次数count，alloc时count + 1，dealloc时count - 1。
/// 当count为0时才回收所有ByteAllocator的区域(b_pos收到start那里)
/// ByteAllocator:
///     1. 不检查dealloc时给出的参数pos位置是否是已经分配的位置(信任调用者)
///         bump_allocator的Cargo.toml引用了allocator依赖(<https://github.com/arceos-org/allocator.git>)，
///         打开链接后，其Cargo.toml中有documentation地址，顺着找到BuddyByteAllocator的dealloc()实现：
///         <https://arceos.org/allocator/src/allocator/buddy.rs.html#43-45>，也没检查pos合法性什么的。
///     2. 中间的一个位置如果alloc后dealloc了，然后又来一个alloc，就算能用中间那个空洞，也不去用，而是继续增长b_pos，
///         只有count减为0，才能回收掉
/// PageAllocator:
///     按上面的注释"For pages area, it will never be freed!"，不考虑页的回收
/// 
/// EarlyAllocator所掌管的内存区域是init时注册给它的，不考虑add_memory()再增加可分配内存区域的情况：
/// 这第三个练习`make run A=exercises/alt_alloc/`，是arceos/modules/alt_axalloc/src/lib.rs里的static GLOBAL_ALLOCATOR
/// 需要初始化一个EarlyAllocator用来实现内存分配，这才用到EarlyAllocator，但是GLOBAL_ALLOCATOR对外提供的add_memory()是unimplemented!()，
/// 所以这里add_memory()也不用实现。
/// 
/// 很多报错情况没管，比如内存不够b_pos和p_pos相互越过的情况
/// 
/// 
/// 这里bump_allocator正经做应该是可以用arceos/modules/bump_allocator/Cargo.toml的那个allocator依赖里的 BuddyByteAllocator
/// 和 BitmapPageAllocator 组合出来的，依赖是写好在Cargo.toml里的，后面可以试试(TODO)。

pub struct EarlyAllocator<const PAGE_SIZE: usize> { // 常量作为泛型参数
    start: usize,
    end: usize, // 可供EarlyAllocator策划分配的区域为[start, end)
    b_pos: usize, // [start, b_pos)是Byte分配
    p_pos: usize, // [p_pos, end)是Page分配
    count: usize, // 现在分配的字节区域个数
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    /// 如果一个fn用来初始化一个 static变量 或者在 const fn 中使用它，则这个fn必须是 const fn，必须在编译期就能跑它。
    /// 在arceos/modules/alt_axalloc/src/lib.rs中，EarlyAllocator::new()在 const fn 中被调用，所以这里必须是 const fn
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = self.end;
    }

    /// 练习3，arceos/modules/alt_axalloc/src/lib.rs里的static GLOBAL_ALLOCATOR，add_memory()是unimplemented!()，所以这里也不用实现
    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        unimplemented!() // 这里为什么没写返回值能过编译？unimplemented!()会调用panic_handler返回never type (`!`)，`!`可以适配任何返回类型
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align = layout.align();
        self.b_pos = (self.b_pos + align - 1) & !(align - 1); // b_pos向上取整对齐后再分配，写(self.b_pos + align - 1) / align * align也行
        let res = unsafe { NonNull::new_unchecked(self.b_pos as *mut u8) };
        self.b_pos += layout.size();
        self.count += 1;
        Ok(res)
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        self.count -= 1;
        if self.count == 0 {
            self.b_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        let size = num_pages * Self::PAGE_SIZE;
        let align = 1 << align_pow2;
        self.p_pos = (self.p_pos - size) & !(align - 1);
        Ok(self.p_pos)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        unimplemented!() // 按上面注释，这个EarlyAllocator，假设页区不会回收
    }

    fn total_pages(&self) -> usize {
        (self.end - self.start) / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / Self::PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / Self::PAGE_SIZE
    }
}
