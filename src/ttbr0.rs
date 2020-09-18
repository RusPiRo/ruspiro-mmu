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

use super::{MmuConfig, config::{TABLE, BLOCK}};

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
/// Based on this and the page table entry type the content/bit settings of a table entry differs.
/// While setting up the MMU we have configured a 4KB granule size. This means at level 1 each page table entry covers
/// a 1GB memory area and has to point to a level 2 descriptor table. Therefore we will cover here the details starting
/// at level 2.
///
/// Level 1 and Level 2 covering 1GB / 2MB respectively
/// |Table entry type - Bits |63|62 61|60|59 |58 52|51  48|47                     30|29       12|11          |1 0|
/// |------------------------|--|-----|--|---|-----|------|-------------------------|-----------|------------|---|
/// | Table entry            |NS|AP   |XN|PXN|     | RES0 | Next level table address [47..12]   |            |1 1|
/// | Block entry            |  Block attributes   | RES0 | Output address [47..30] | RES0      | Block attr.|0 1|
///
/// Level 3 does not allow for further table references, this is the memory page level of the desired granule (4KB)
/// |Table entry type - Bits |63|62 61|60|59 |58 52|51  48|47                     30|29       12|11          |1 0|
/// |------------------------|--|-----|--|---|-----|------|-------------------------|-----------|------------|---|
/// | Page entry             |  Page Attributes    | RES0 | Output address [47..12]             | Page attr. |1 1|
///
/// The upper and lower block/page attributes are the same on each level of the translation tables. They only differ
/// based on the executed translation stage. The different stages are only relevent in case the translation happens
/// within "user level". This means the first translation stage will map the memory into an intermediate physical
/// address, where the second stage will map this IPA into the real physical address. However, the current RusPiRo MMU
/// setup is configured to only use a one stage translation process always immediately resulting in a physical address.
///
///  Upper Attributes (Stage 1)
/// |63     59|58     55| 54 | 53  |52 |
/// |---------|---------|----|-----|---|
/// | ignored | ignored | XN | PXN | C |
///
/// Bits 63..55 are ignored. The difference here is that bit 63..60 may be used by the MMU implementation of the Chip
/// and bit 58..55 may be used by the actual software
///
/// Bit  | Description
/// -----|-------------
///  XN  | eXecute Never bit determining whether the memory region is executable or not.
///  PXN | Priviliged eXecute Never bit determines whether the memory region is executable in EL1. In EL2/EL3 this bit is RES0
///  C   | Contigues hint bit indicating that this table entry is one of a contigues sets of entries and might be cached
///      | together with the other ones
///
/// Lower Attributes (Stage 1)
/// |11|10  |9  8|7  6|5   |4       2|
/// |--|----|----|----|----|---------|
/// |nG| AF | SH | AP | NS | MemAttr |
///
/// Bit      | Description
/// ---------|-------------
///  nG      | not Global bit determines whether this entry is globally valid or only for the current ASID value. This  bit is only valid in EL1 & EL0
///  AF      | Access Flag bit
///  SH      | Shareability flag
///  AP      | data Access Permission bits for AP[2..1], AP[0] is not defined in the TLB entries
///  NS      | Non-Secure bit specifies whether the output address is in secure or non-secure address map.
///  MemAttr | Stage 1 memory attributes - index into MAIR_ELx register
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
            (TABLE::NS::SET | TABLE::TYPE::VALID).raw_value() | (level2_addr_1 as u64)
        );
        write_volatile(
            &mut MMU_CFG.ttlb_lvl1[1] as *mut u64,
            (TABLE::NS::SET | TABLE::TYPE::VALID).raw_value() | (level2_addr_2 as u64)
        );

        // the entries in level 2 (covering 2MB each) contain the specific memory attributes for this memory area
        // first entries up to an initial fixed address covering 2Mb are "normal" memory
        for i in 0..4 {
            // 1:1 memory mapping with it's attributes
            write_volatile(
                &mut MMU_CFG.ttlb_lvl2[i],
                (
                    BLOCK::NS::SET
                    | BLOCK::AF::SET
                    | BLOCK::SH::INNER
                    | BLOCK::MEMATTR::MAIR4
                    | BLOCK::TYPE::VALID
                ).raw_value() | (i as u64) << 21
            ); // block entry
        }

        // get the blocj that covers the VideoCore memory and configure it to be non-cacheable from ARM point of view
        let vc_start_block = (vc_mem_start >> 21) as usize;
        let vc_end_block = ((vc_mem_start + vc_mem_size) >> 21) as usize;
        for i in vc_start_block..vc_end_block {
            // 1:1 memory mapping with it's attributes
            write_volatile(
                &mut MMU_CFG.ttlb_lvl2[i],
                (
                    BLOCK::AF::SET
                    | BLOCK::SH::INNER
                    | BLOCK::MEMATTR::MAIR3
                    | BLOCK::TYPE::VALID
                ).raw_value() | (i as u64) << 21
            ); // block entry
        }

        // entries from 0x3F00_0000 to 0x4020_0000 are "device" memory
        for i in 504..513 {
            // 1:1 memory mapping with it's attributes
            write_volatile(
                &mut MMU_CFG.ttlb_lvl2[i],
                (
                    BLOCK::AF::SET
                    | BLOCK::SH::INNER
                    | BLOCK::MEMATTR::MAIR0
                    | BLOCK::TYPE::VALID
                ).raw_value() | (i as u64) << 21
            ); // block entry
        }

        llvm_asm!("dsb   ishst");
    }

    &MMU_CFG.ttlb_lvl1[0] as *const u64
}
