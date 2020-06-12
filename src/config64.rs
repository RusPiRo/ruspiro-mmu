//! # MMU Configuration Constants
//! 

pub const SECTION_SIZE: usize = 0x20_0000; // 2MB section size
pub const SECTION_MASK: usize = SECTION_SIZE - 1;
pub const SECTION_SHIFT: usize = 21;
pub const PAGE_SIZE: usize = 4096; // 4kB page size
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;