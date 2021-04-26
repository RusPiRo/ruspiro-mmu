/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 *
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/

//! # TTBR0 Configuration
//!
//! Physical address space mapping
//!

use core::ptr::write_volatile;

use super::{
  config::{TTLB_BLOCKPAGE, TTLB_TABLE},
  MmuConfig,
};

/// level 1 translation table, each entry covering 1GB of memory
/// level 2 translation table, each entry covering 2MB of memory
/// level 3 translation table, each entry covering 4kB of memory
static mut MMU_CFG: MmuConfig = MmuConfig {
  ttlb_lvl1: [0; 512],
  ttlb_lvl2: [0; 1024],
  //ttlb_lvl3: [0; 2560],
};

/// Perform the actual page table configuration to ensure 1:1 memory mapping (virtual -> physical) with the desired
/// attributes of the lower virtual memory region - typically application space - ranging from
/// 0x0000_0000_0000_0000 to 0x0000_007F_FFFF_FFFF. The upper boundry is given by the SCTLR_EL1-T1SZ register
/// => 2^(64-T1SZ) - 1. The upper bound is only valid for EL1/EL0. EL3/EL2 does only have a TTBR0 table to cover
/// virtual to physical address mapping
///
/// # Safety
/// A call to this initial MMU setup and configuration should always be done only once from
/// the main core booting up first only. As long as the MMU is not up and running there is no way
/// to secure access with atomic operations as they require the MMU to be active - otherwise the usage of
/// atomics will simply hang the core
pub unsafe fn setup_translation_tables(
  core: u32,
  vc_mem_start: u32,
  vc_mem_size: u32,
) -> *const u64 {
  // initial MMU page table setup only on core 0!
  if core == 0 {
    // this first attempt provides very huge configuration blocks, meaning we
    // setup the smallest unit to cover 2Mb blocks of memory sharing the same memory attributes

    let level2_addr_1 = &MMU_CFG.ttlb_lvl2[0] as *const u64;
    let level2_addr_2 = &MMU_CFG.ttlb_lvl2[512] as *const u64;

    // the entries in level 1 (covering 1GB each) need to point to the next level table
    // that contains more granular config
    write_volatile(
      &mut MMU_CFG.ttlb_lvl1[0] as *mut u64,
      (TTLB_TABLE::NS::SET
        | TTLB_TABLE::TYPE::VALID
        | TTLB_TABLE::ADDR::from_raw(level2_addr_1 as u64))
      .raw_value(),
    );
    write_volatile(
      &mut MMU_CFG.ttlb_lvl1[1] as *mut u64,
      (TTLB_TABLE::NS::SET
        | TTLB_TABLE::TYPE::VALID
        | TTLB_TABLE::ADDR::from_raw(level2_addr_2 as u64))
      .raw_value(),
    );

    // the entries in level 2 (covering 2MB each) contain the specific memory attributes for this memory area
    // first entries up to an initial fixed address (VideoCore Memory start) covering 2Mb are "normal" memory
    // get the block that covers the VideoCore memory
    let vc_start_block = (vc_mem_start >> 21) as usize;
    let vc_end_block = ((vc_mem_start + vc_mem_size) >> 21) as usize;
    for i in 0..vc_start_block {
      // 1:1 memory mapping with it's attributes
      write_volatile(
        &mut MMU_CFG.ttlb_lvl2[i],
        (TTLB_BLOCKPAGE::NS::SET
          | TTLB_BLOCKPAGE::AF::SET
          | TTLB_BLOCKPAGE::SH::INNER
          | TTLB_BLOCKPAGE::MEMATTR::MAIR4
          | TTLB_BLOCKPAGE::TYPE::BLOCK
          | TTLB_BLOCKPAGE::ADDR::from_raw((i as u64) << 21))
        .raw_value(),
      ); // block entry
    }

    // Configure the VC memory region to be non-cacheable from ARM point of view
    for i in vc_start_block..vc_end_block {
      // 1:1 memory mapping with it's attributes
      write_volatile(
        &mut MMU_CFG.ttlb_lvl2[i],
        (TTLB_BLOCKPAGE::AF::SET
          | TTLB_BLOCKPAGE::SH::INNER
          | TTLB_BLOCKPAGE::MEMATTR::MAIR3
          | TTLB_BLOCKPAGE::TYPE::BLOCK
          | TTLB_BLOCKPAGE::ADDR::from_raw((i as u64) << 21))
        .raw_value(),
      ); // block entry
    }

    // if there is a memory block left after VC memory up to the device memory
    // maintain this area as normal memory
    for i in vc_end_block..504 {
      // 1:1 memory mapping with it's attributes
      write_volatile(
        &mut MMU_CFG.ttlb_lvl2[i],
        (TTLB_BLOCKPAGE::NS::SET
          | TTLB_BLOCKPAGE::AF::SET
          | TTLB_BLOCKPAGE::SH::INNER
          | TTLB_BLOCKPAGE::MEMATTR::MAIR4
          | TTLB_BLOCKPAGE::TYPE::BLOCK
          | TTLB_BLOCKPAGE::ADDR::from_raw((i as u64) << 21))
        .raw_value(),
      ); // block entry
    }

    // entries from 0x3F00_0000 to 0x4020_0000 are "device" memory
    for i in 504..513 {
      // 1:1 memory mapping with it's attributes
      write_volatile(
        &mut MMU_CFG.ttlb_lvl2[i],
        (TTLB_BLOCKPAGE::AF::SET
          | TTLB_BLOCKPAGE::SH::INNER
          | TTLB_BLOCKPAGE::MEMATTR::MAIR0
          | TTLB_BLOCKPAGE::TYPE::BLOCK
          | TTLB_BLOCKPAGE::ADDR::from_raw((i as u64) << 21))
        .raw_value(),
      ); // block entry
    }

    llvm_asm!("dsb   ishst");
  }

  &MMU_CFG.ttlb_lvl1[0] as *const u64
}
