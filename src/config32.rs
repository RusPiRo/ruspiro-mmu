//! # MMU Configuration Constants
//! 

pub const SECTION_SIZE: usize = 0x10_0000; // 1MB section size
pub const SECTION_MASK: usize = SECTION_SIZE - 1;
pub const SECTION_SHIFT: usize = 20;
pub const PAGE_SIZE: usize = 4096; // 4kB page size
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;