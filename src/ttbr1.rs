/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 * 
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/

//! # TTBR1 Configuration
//!
//! Virtual address space mapping
//!

use core::ptr::write_volatile;

use super::{MmuConfig, config::TABLE};

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
/// 0xFFFF_FF80_0000_0000 to 0xFFFF_FFFF_FFFFF_FFFF. The upper boundry is given by the SCTLR_EL1-T1SZ register
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
pub unsafe fn setup_translation_tables(core: u32) -> *const u64 {
    // initial MMU page table setup only on core 0!
    if core == 0 {
        // The TTBR1 setting starts with an initial configuartion of valid entries in level 1 covering 1 GB memory space
        // each. The VA mapping will start at the lowest possible address and works forward while handing out virtual
        // addresses. The VA are configured 2MB block wise in level 2 only. Even if the mapped physical memory is not
        // requiring this.
        // this is the address that is stored in th TTBR1 register. From here the GB sized index starts.
        // so maintaining one entry that could cover 1GB starting at 0xFFFF_FFFF_FFFF_FFFF. This entry is a table
        // entry and point to the table where the block configuration is stored, each block covering 2MB of memory
        let level2_addr = &MMU_CFG.ttlb_lvl2[0] as *const u64;
        write_volatile(
            &mut MMU_CFG.ttlb_lvl1[511] as *mut u64,
            (TABLE::NS::SET | TABLE::TYPE::VALID).raw_value() | (level2_addr as u64),
        );

        // we will not maintain any block entry at the beginning as those are maintained when memory mapping
        // happens and a virtual address is required to be mapped to a physical one with specific memory
        // attributes. As the block entries are all invalid at the beginning any memory access would lead to a access
        // fault
        llvm_asm!("dsb   ishst");
    }

    &MMU_CFG.ttlb_lvl1[0] as *const u64
}

/// Maintain the TTBR1 translation table pages to provide the virtual address and it's occupied space with the proper
/// memory attributes.
/// # Safety
/// This is safe if the address given has been returned by `alloc::alloc(...)` function and spans the size passed.
/// It will panic if the TTBR1 configuration does not allow to maintain any further VA address range
/// # TODO
/// actually it maintains a whole 2MB block for any size given. This is quite wastefull and should be changed to do
/// page size maintenance incorporating the number of pages to be configured based on the size given
pub unsafe fn maintain_pages(origin: *mut u8, _size: usize, attributes: u64) -> *mut u8 {
    // page maintenance is done at the beginning on 2MB block level only. This is quite ok as
    // we have plenty of virtual memory we can map to physical one. So even the mapped memory falls into the same
    // physical 2MB region we can use a different 2MB virtual block and virtual address from this block.
    // This is actually wasting lot's of virtual address space and table entries but for the time beeing we do not
    // expect many regions to be maintained.

    // 1. find the next free block in the page table
    let block_entry = MMU_CFG
        .ttlb_lvl2
        .iter_mut()
        .enumerate()
        .find(|(_, entry)| **entry == 0);

    if let Some((idx, entry)) = block_entry {
        // we found a block entry we can use
        // maintain the entry in the translation table
        let tlb_value = 0b1 << 63
                | attributes // memory attributes
                | ((origin as u64) & !0x1F_FFFF) // physical block start address 
                | 1 << 10 // access flag
                | 0b01;
        write_volatile(&mut *entry, tlb_value);
        // once the table has been updated we need to invalidate this entry
        let entry_addr = entry as *const u64 as usize;
        llvm_asm!("dsb   ishst
                dsb   ish
                isb
                dc civac, $0"::"r"(entry_addr)::"volatile");
        // calculate the virtual address for this entry based on the current block we are using
        let mut va = 0xFFFF_FFFF_FFFF_FFFF - (((512 - idx) << 21) - 1);
        va |= origin as usize & 0x1F_FFFF;

        va as *mut u8
    } else {
        // if there is no more virtual address block available we need to panic!
        panic!("all VA addresses occupied");
    }
}
