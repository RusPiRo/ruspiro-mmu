/***********************************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache 2.0
 **********************************************************************************************************************/

//! # MMU maintenance
//!
use core::ptr::*;
use ruspiro_register::system::*;
use ruspiro_console::*;

use crate::config::*;

#[repr(C, align(4096))]
struct MmuConfig {
    ttlb_lvl0: [u64; 512], // the size of 512 ensures the next level table start is also aligned to 4kB
    // page level ttlb entries - for PoC just enough to map one 2MB region.
    // should there be always space from the stack allocated even if we most likely will never maintain all
    // entries on the page level (would require 512*512*size_of::<u64>()). But getting this memory dynamically from
    // the heap is also questionnable as it is then scattered around in memory that would also require
    // specific memory attributes!? Let's answer those question when we see the page level is working at all...
    ttlb_lvl2: [u64; 1024], // the size of 1024 ensures the next level table start is also aligned to 4kB
    ttlb_lvl1: [u64; 513],
    
}

/// level 0 translation table, each entry covering 1GB of memory
/// level 1 translation table, each entry covering 2MB of memory
static mut MMU_CFG: MmuConfig = MmuConfig {
    ttlb_lvl0: [0; 512],
    ttlb_lvl2: [0; 1024],
    ttlb_lvl1: [0; 513],
};

/// Initialize the MMU. This configures an initial 1:1 mapping accross the whole available
/// memory of the Raspberry Pi. Only the memory region from 0x3F00_0000 to 0x4002_0000 is configured
/// as device memory as this is the area the memory mapped peripherals and the core mailboxes are
/// located at.
pub fn initialize_mmu(core: u32) {
    // the mmu configuration depents on the exception level we are running in
    let el = currentel::read(currentel::EL::Field).value();

    // disable MMU before changing any settings and re-activating
    match el {
        1 => disable_mmu_el1(),
        2 => disable_mmu_el2(),
        _ => unimplemented!(),
    }

    // setup ttlb entries - this is only needed once on the main core
    // as all cores share the same physical memory
    if core == 0 {
        setup_page_tables();
    }

    match el {
        1 => initialize_mmu_el1(),
        2 => initialize_mmu_el2(),
        _ => unimplemented!(),
    }
}

/// Disable the MMU. This keeps the current mapping table configuration untouched.
#[allow(dead_code)]
pub fn disable_mmu() {
    // the mmu configuration depents on the exception level we are running in
    let el = currentel::read(currentel::EL::Field).value();
    match el {
        1 => disable_mmu_el1(),
        2 => disable_mmu_el2(),
        _ => unimplemented!(),
    }
    // let 2 cycles pass with a nop to settle the MMU after disabling
    nop();
    nop();
    dsb();
    isb();
}

fn initialize_mmu_el1() {
    // configure the MAIR (memory attribute) variations we will support
    // those entries are referred to as index in the memeory attributes of the
    // table entries
    mair_el1::write(
        mair_el1::MAIR0::NGNRNE
            | mair_el1::MAIR1::NGNRE
            | mair_el1::MAIR2::GRE
            | mair_el1::MAIR3::NC
            | mair_el1::MAIR4::NORM,
    );

    // set the ttlb base address, this is where the memory address translation
    // table walk starts
    let ttlb_base = unsafe { (&MMU_CFG.ttlb_lvl0[0] as *const u64) as u64 };
    ttbr0_el1::write(ttbr0_el1::BADDR::with_value(ttlb_base));

    // configure the TTLB attributes
    tcr_el1::write(
        tcr_el1::T0SZ::with_value(25)
            | tcr_el1::EPD0::ENABLE
            | tcr_el1::IRGN0::NM_INC //NM_IWB_RA_WA
            | tcr_el1::ORGN0::NM_ONC // NM_OWB_RA_WA
            | tcr_el1::SH0::OS //IS
            | tcr_el1::TG0::_4KB
            | tcr_el1::T1SZ::with_value(25)
            | tcr_el1::EPD1::DISABLE
            | tcr_el1::IRGN1::NM_INC //NM_IWB_RA_WA
            | tcr_el1::ORGN1::NM_ONC //NM_OWB_RA_WA
            | tcr_el1::SH1::OS //IS
            | tcr_el1::TG1::_4KB
            | tcr_el1::IPS::_32BITS
            | tcr_el1::TBI0::IGNORE,
    );

    // set the SCTRL_EL1 to activate the MMU
    sctlr_el1::write(
        sctlr_el1::M::ENABLE
            | sctlr_el1::A::DISABLE
            | sctlr_el1::C::ENABLE
            | sctlr_el1::SA::DISABLE
            | sctlr_el1::I::ENABLE,
    );

    // let 2 cycles pass with a nop to settle the MMU
    nop();
    nop();

    /*unsafe {
        llvm_asm!("tlbi  alle1is");
    }*/
}

fn disable_mmu_el1() {
    sctlr_el1::write(sctlr_el1::M::DISABLE | sctlr_el1::C::DISABLE | sctlr_el1::I::DISABLE);
}

fn initialize_mmu_el2() {
    // configure the MAIR (memory attribute) variations we will support
    // those entries are referred to as index in the memeory attributes of the
    // table entries
    mair_el2::write(
        mair_el2::MAIR0::NGNRNE
            | mair_el2::MAIR1::NGNRE
            | mair_el2::MAIR2::GRE
            | mair_el2::MAIR3::NC
            | mair_el2::MAIR4::NORM,
    );

    // set the ttlb base address, this is where the memory address translation
    // table walk starts
    let ttlb_base = unsafe { (&MMU_CFG.ttlb_lvl0[0] as *const u64) as u64 };
    ttbr0_el2::write(ttbr0_el2::BADDR::with_value(ttlb_base));

    // configure the TTLB attributes
    tcr_el2::write(
        tcr_el2::T0SZ::with_value(25)
            | tcr_el2::IRGN0::NM_INC //NM_IWB_RA_WA
            | tcr_el2::ORGN0::NM_ONC //NM_OWB_RA_WA
            | tcr_el2::SH0::OS //IS
            | tcr_el2::TG0::_4KB
            | tcr_el2::PS::_32BITS
            | tcr_el2::TBI::IGNORE,
    );

    hcr_el2::write(hcr_el2::DC::DISABLE | hcr_el2::VM::DISABLE);

    // set the SCTRL_EL2 to activate the MMU
    sctlr_el2::write(
        sctlr_el2::M::ENABLE
            | sctlr_el2::A::DISABLE
            | sctlr_el2::C::ENABLE
            | sctlr_el2::SA::DISABLE
            | sctlr_el2::I::ENABLE,
    );

    // let 2 cycles pass with a nop to settle the MMU
    nop();
    nop();

    /*unsafe {
        llvm_asm!("tlbi  alle2is");
    }*/
}

fn disable_mmu_el2() {
    sctlr_el2::write(sctlr_el2::M::DISABLE | sctlr_el2::C::DISABLE | sctlr_el2::I::DISABLE);
}

/// Perform the actual page table configuration to ensure 1:1 memory mapping with the desired
/// attributes.
/// 
/// Based on this and the page table entry type the content/bit settings of a table entry differs.
/// While setting up the MMU we have configured a 4KB granule size. This means at level 0 each page table entry covers 
/// a 1GB memory area and has to point to a level 1 descriptor table. Therefore we will cover here the details starting 
/// at level 1.
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
///  nG      | not Global bit determines whether this entry is globally valid or only for the current ASID value. This bit is only valid in EL1 & EL0
///  AF      | Access Flag bit
///  SH      | Shareability flag
///  AP      | data Access Permission bits for AP[2..1], AP[0] is not defined in the TLB entries
///  NS      | Non-Secure bit specifies whether the output address is in secure or non-secure address map.
///  MemAttr | Stage 1 memory attributes - index into MAIR_ELx register
///
/// # Safety
/// A call to this initial MMU setup and configuration should always be called only once and from
/// the main core booting up first only. As long as the MMU is not up and running there is no way
/// to secure access with atmic operations as they require the MMU to not hang the core
fn setup_page_tables() {
    // initial MMU page table setup
    // this first attempt provides very huge configuration blocks, meaning we
    // setup the smallest unit to cover 2Mb blocks of memory sharing the same memory attributes
    unsafe {
        let level1_addr_1 = &MMU_CFG.ttlb_lvl1[0] as *const u64;
        let level1_addr_2 = &MMU_CFG.ttlb_lvl1[512] as *const u64;

        // the entries in level 0 (covering 1GB each) need to point to the next level table
        // that contains more granular config
        write_volatile(&mut MMU_CFG.ttlb_lvl0[0] as *mut u64, 0x1 << 63 | (level1_addr_1 as u64) | 0b11);
        write_volatile(&mut MMU_CFG.ttlb_lvl0[1] as *mut u64, 0x1 << 63 | (level1_addr_2 as u64) | 0b11);

        // the entries in level 1 (covering 2MB each) contain the specific memory attributes for
        // this memory area
        // first entries up to 0x3F00_0000 are "normal" memory
        for i in 0..504 {
            // 1:1 memory mapping with it's attributes
            // AF = 1 << 10, SH = 3 << 8, MAIR index = 4 << 2
            write_volatile(&mut MMU_CFG.ttlb_lvl1[i],
                (i as u64 * 0x20_0000) // | 0x710 
                | 1 << 10 // access flag
                | 0b11 << 8 // shareable flag
                | 0b100 << 2 // MAIR 4 -> NORMAL memory
                | 0b01); // block entry
        }

        // entries from 0x3F00_0000 to 0x4020_0000 are "device" memory
        for i in 504..513 {
            // 1:1 memory mapping with it's attributes
            // AF = 1 << 10, SH = 0 << 8, MAIR index = 0 << 2
            write_volatile(&mut MMU_CFG.ttlb_lvl1[i],
                (i as u64 * 0x20_0000) // | 0x400 
                |  1 << 10 // access flag
                | 0b01); // block entry
        }

        llvm_asm!("dsb   ishst");
    }
}

/// Maintain the page attribute within the corresponding TTLB's 
pub fn maintain_pages(addr: *mut u8, page_from: usize, page_count: usize, page_attributes: u64) {
    info!("maintain page from {} to {} for address {:#x?}", page_from, page_from + page_count, addr);
    // as POC we always assume we will have only maximum 2 2MB memory block that is maintaining it's page level
    // attributes
    // get the 2MB block index that we need to start maintaining
    let block_idx_1 = page_from / 512;
    let block_idx_2 = (page_from + page_count) / 512;
    info!("start block {} - end block {}", block_idx_1, block_idx_2);
    unsafe {
        // get the page table address this block should point to
        let level2_page_addr = &MMU_CFG.ttlb_lvl2[0] as *const u64;
        info!("level2 address: {:#x?}", level2_page_addr);
        // change the actual block entry from block to table type and set the page table address that contain the page
        // configurations
        // remember the upper and lower block attributes to use them to maintain the pages that are part of the block
        // but not requested to change their attributes
        let block_attributes = MMU_CFG.ttlb_lvl1[block_idx_1] & 0xFFF0000000000FFC;
        // get the offset from the block start of the requested page to maintain
        let block_page_start = page_from - block_idx_1 * 512;
        info!("maintain pages 0 - {} with current settings", block_page_start);
        for i in 0..block_page_start {
            //info!("page address: {:#x?}", (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000));
            write_volatile(&mut MMU_CFG.ttlb_lvl2[i],
                block_attributes | (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000) | 0b11
            );
        }
        // maintain the number of pages with the requested attributes
        info!("maintain pages {} - {} with requested settings", block_page_start, block_page_start + page_count);
        for i in block_page_start..block_page_start + page_count {
            //info!("page {} - address: {:#x?}", i, (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000));
            write_volatile(&mut MMU_CFG.ttlb_lvl2[i],
                page_attributes | (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000) | 0b11
            );
        }

        // finally maintain the remainder (if any) from the last requested page to the end of the block with the 
        // original block attributes
        let block_page_end = (block_idx_2 + 1) * 512;
        info!("maintain remainder of pages {} - {}", block_page_start + page_count, block_page_end);
        for i in block_page_start + page_count..block_page_end {
            //info!("page address: {:#x?}", (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000));
            write_volatile(&mut MMU_CFG.ttlb_lvl2[i],
                block_attributes | (i as u64 * 0x1000 + block_idx_1 as u64 * 0x20_0000) | 0b11
            );
        }

        // maintain the level 1 to be a table entry pointing to the pages after providing the page table entries
        //MMU_CFG.ttlb_lvl1[block_idx_1] = 0x1 << 63 | (level2_page_addr as u64) | 0b11;
        write_volatile(&mut MMU_CFG.ttlb_lvl1[block_idx_1],
            0x0 << 63 | (level2_page_addr as u64) | 0b11
        );

        // if the pages that required maintenance caused an overlap into another block we need to maintain the
        // corresponding block entry as well as a table entry pointing to the page table
        if block_idx_1 != block_idx_2 {
            info!("block overlap - maintain second block");
            // maintain the level 1 to be a table entry pointing to the pages for it's blocks
            let level2_page_addr = &MMU_CFG.ttlb_lvl2[512] as *const u64;
            write_volatile(&mut MMU_CFG.ttlb_lvl1[block_idx_2],
                0x1 << 63 | (level2_page_addr as u64) | 0b11
            );
        }

        // ensure all data is written for the next statement
        llvm_asm!("dsb   ishst");
        // flush all stage 1 EL1 TLB entries
        llvm_asm!("tlbi  VMALLE1");
    }
}
