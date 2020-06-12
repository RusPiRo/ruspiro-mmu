/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **********************************************************************************************************************/
 #![doc(html_root_url = "https://docs.rs/ruspiro-mmu/0.1.0")]
 #![no_std]
 #![feature(llvm_asm)]
//! # Raspberry Pi MMU
//! 
//! This crate provide functions to manage the use of the Raspberry Pi Memory Management Unit (MMU)
//!

#[cfg_attr(target_arch = "aarch64", path = "mmu64.rs")]
#[cfg_attr(target_arch = "arm", path = "mmu32.rs")]
mod mmu;
pub use mmu::*;

#[cfg_attr(target_arch = "aarch64", path = "config64.rs")]
#[cfg_attr(target_arch = "arm", path = "config32.rs")]
pub mod config;

/// Align a given address to the current MMU page size
pub fn page_align(addr: usize) -> usize {
    (addr + config::PAGE_MASK) & !config::PAGE_MASK
}