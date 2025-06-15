//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{AllocResult, BaseAllocator, ByteAllocator, TlsfByteAllocator};
use axlog::{ax_println, error};
use core::ptr::NonNull;
use core::alloc::Layout;

const FREE_MEMORY_START: usize = 0xffffffc08026e000;
const FREE_MEMORY_END: usize = 0xffffffc088000000;
// 0xffffffc088000000 - 0xffffffc08026e000 = 128584KB，约 125MB

const TLSF_POOL_SIZE: usize = (FREE_MEMORY_END - FREE_MEMORY_START) - (512 * 1024 + 150000);


// 见 ./思路.md
pub struct LabByteAllocator {
    tlsf: TlsfByteAllocator,
    u8_start: usize,
    u8_end: usize
}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            tlsf: TlsfByteAllocator::new(),
            u8_start: 0,
            u8_end: 0
        }
    }
}

impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        // ax_println!("LabByteAllocator.init(): start: {:#x}, size: {}KB", start, size / 1024); // start为0xffffffc08026e000, 初始只分配给ByteAllocator 32KB
        self.tlsf.init(start, TLSF_POOL_SIZE);
        self.u8_start = start + TLSF_POOL_SIZE;
        self.u8_end = FREE_MEMORY_END;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // 好像是因为unimplemented!()展开后返回值是!(never type，永不返回)，所以编译器知道该函数永远不会返回，所以后面有个分号也没报()和AllocResult不匹配
        unimplemented!();
    }
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        /* if align == 8 && size > 384 {
            ax_println!("LabByteAllocator.alloc() align == 8 && size > 384: size: {} align: {}", size, align);
        } */
        // ax_println!("LabByteAllocator.alloc() size: {} align: {}", size, align);

        if align == 8 {
            let res = self.tlsf.alloc(layout);
            match res {
                Ok(ptr) => {
                    return AllocResult::Ok(ptr);
                },
                Err(err) => {
                    return AllocResult::Err(allocator::AllocError::NoMemory);
                }
            }
        }
        
        // 分配[u8]，直接给提前准备好的一段
        let alloc_u8_end = self.u8_start + size;
        if alloc_u8_end > FREE_MEMORY_END { panic!("exceed free memory end"); }
        AllocResult::Ok(NonNull::new(self.u8_start as *mut u8).unwrap())
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        
    }

    fn total_bytes(&self) -> usize {
        FREE_MEMORY_END - FREE_MEMORY_START
    }

    fn used_bytes(&self) -> usize {
        unimplemented!();
    }

    fn available_bytes(&self) -> usize {
        unimplemented!();
    }
}
