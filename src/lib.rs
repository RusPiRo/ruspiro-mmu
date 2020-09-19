/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 *
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/
#![doc(html_root_url = "https://docs.rs/ruspiro-mmu/||VERSION||")]
#![cfg_attr(not(any(test, doctest)), no_std)]
#![feature(llvm_asm)]
#![cfg(target_arch = "aarch64")]

//! # RusPiRo MMU API
//!
//! This crate provide the API to configure and maintain the Raspberry Pi Memory Management Unit. On Raspberry Pi a
//! configured and active MMU is a prerequisit to use any atomic operations.
//!

use ruspiro_arch_aarch64::{register::currentel, register_field, register_field_values};

mod config;
mod el1;
mod el2;
mod macros;
mod ttbr0;
mod ttbr1;
pub use config::TTLB_BLOCKPAGE;

/// Initialize the MMU. This configures an initial 1:1 mapping accross the whole available
/// memory of the Raspberry Pi. Only the memory region from 0x3F00_0000 to 0x4002_0000 is configured
/// as device memory as this is the area the memory mapped peripherals and the core mailboxes are
/// located at.
pub fn initialize(core: u32, vc_mem_start: u32, vc_mem_size: u32) {
    // the mmu configuration depends on the exception level we are running in
    let el = currentel::read(currentel::EL::Field).value();

    // disable MMU before changing any settings and re-activating
    match el {
        1 => el1::disable_mmu(),
        2 => el2::disable_mmu(),
        _ => unimplemented!(),
    }

    // setup translation table entries
    let ttlb0_base_addr =
        unsafe { ttbr0::setup_translation_tables(core, vc_mem_start, vc_mem_size) as u64 };
    match el {
        1 => {
            let ttlb1_base_addr = unsafe { ttbr1::setup_translation_tables(core) as u64 };
            el1::enable_mmu(ttlb0_base_addr, ttlb1_base_addr);
        }
        2 => el2::enable_mmu(ttlb0_base_addr),
        _ => unimplemented!(),
    }
}

/// Map a given address to a virtual address with the specified memory attributes.
/// TODO: Memory attributes shall be a specific allowed set only - create a new type for this!
/// # Safety
/// This is safe if the MMU has been configured already. Also the given raw pointer need to point to an
/// address provided from a call to `alloc::alloc(...)` with at least `size` bytes and is aligned to the actual
/// page size boundries.
/// # Hint
/// If the MMU is not configured to use the TTBR1 virtual address mapping this call has no effect and the returned
/// address can not being used.
pub unsafe fn map_memory(origin: *mut u8, size: usize, attributes: u64) -> *mut u8 {
    // the mmu configuration depends on the exception level we are running in
    let el = currentel::read(currentel::EL::Field).value();
    if el == 1 {
        ttbr1::maintain_pages(origin, size, attributes)
    } else {
        origin
    }
}

/// Align a given address/size to the next page boundary based on MMU config
pub fn page_align(addr: usize) -> usize {
    (addr + config::PAGE_MASK) & !config::PAGE_MASK
}

pub fn page_size() -> usize {
    config::PAGE_SIZE
}

#[repr(C, align(4096))]
struct MmuConfig {
    /// TLB Level 1 entries will cover a memory range of 1GB each. For a Raspberry Pi we would only need 2 entries on
    /// this level, however, we would like to have the subsequent tables to start as 4kb aligned address, so reserving
    /// 512 entries here
    ttlb_lvl1: [u64; 512],
    /// TLB Level 2 entries will cover a memory range of 2MB each, so to maintain entries for the first 1GB of the
    /// Raspberry Pi 512 entries would be enough, however we would need to map the peripheral address space as well and
    /// they are above the 1GB mark but not greater than 2MB, so 513 entries in total would be enough. Nevertheless any
    /// memory located after the table shall be page aligned (4kb) we will add entries do keep the overall structure
    /// size fitting exactly into a multiple of a page and to align the following table to a 4kb boundry
    ttlb_lvl2: [u64; 1024],
    /*// TLB Level 3 entries will cover a memory range of 4kB each. So to be able to maintain memory attributes on this
    /// granule level for every memory block we would need 512*512 entries. That's quite a huge amount of memory that is
    /// most likely wasted, as there will be only a very small amount ob blocks that might require splitting into pages
    /// from the tlb configuration point of view. So we would start with 3 blocks beeing able to be maintained on this
    /// granule level which makes 5*512 entries and gives the overall structure a size of a multiple of a page
    //ttlb_lvl2: [u64; 2560],*/
} // total size : 6kB
