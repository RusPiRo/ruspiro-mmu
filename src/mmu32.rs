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

#[repr(align(0x4000))]
struct MmuConfig {
    ttlb: [u32; 4096],
}

/// translation table in aarch32
static mut MMU_CFG: MmuConfig = MmuConfig { ttlb: [0; 4096] };

/// Initialize the MMU. This configures an initial 1:1 mapping accross the whole available
/// memory of the Raspberry Pi. Only the memory region from 0x3F00_0000 to 0x4002_0000 is configured
/// as device memory as this is the area the memory mapped peripherals and the core mailboxes are
/// located at.
pub fn initialize_mmu(core: u32) {
    // disbale the MMU before changing any configuration settings
    disable_mmu();

    // enable CPUECTRL and ACTLR writes using the ACTLR register
    actlr::write(actlr::CPUACTLR::READWRITE | actlr::CPUECTLR::READWRITE);

    // write TTBCR
    ttbcr::set(0);
    isb();

    if core == 0 {
        setup_page_tables();
    }

    // set ttlb base table address
    //let ttlb_base = TTBASE | 0x06a;
    let ttlb_base = unsafe { (&MMU_CFG.ttlb[0] as *const u32) as u32 }; // | 0x6a; // what's 0x6a?
    ttbr0::write(
        ttbr0::TTB0::with_value(ttlb_base >> 14)
            | ttbr0::S::SHAREABLE
            | ttbr0::RGN::NORM_O_NC //NORM_OWB_NWAC //NORM_O_NC
            | ttbr0::NOS::OUTER
            | ttbr0::IRGN1::with_value(0b0)
            | ttbr0::IRGN0::with_value(0b0),
    );

    // set MMU domains
    dacr::write(
        dacr::D0::MANAGER
            | dacr::D1::MANAGER
            | dacr::D2::MANAGER
            | dacr::D3::MANAGER
            | dacr::D4::MANAGER
            | dacr::D5::MANAGER
            | dacr::D6::MANAGER
            | dacr::D7::MANAGER
            | dacr::D8::MANAGER
            | dacr::D9::MANAGER
            | dacr::D10::MANAGER
            | dacr::D11::MANAGER
            | dacr::D12::MANAGER
            | dacr::D13::MANAGER
            | dacr::D14::MANAGER
            | dacr::D15::MANAGER,
    );

    // enable the MMU, instruction + data cache
    // SCTLR register
    sctlr::write(sctlr::M::ENABLE | sctlr::I::ENABLE | sctlr::C::ENABLE);

    // let 2 cycles pass with a nop to settle the MMU
    nop();
    nop();
    dsb();
    isb();
}

/// Disable the MMU. This keeps the current mapping table configuration untouched.
#[allow(dead_code)]
pub fn disable_mmu() {
    // disable the MMU, instruction + data cache
    // SCTLR register
    sctlr::write(sctlr::M::DISABLE | sctlr::I::DISABLE | sctlr::C::DISABLE);
    // let 2 cycles pass with a nop to settle the MMU
    nop();
    nop();
    dsb();
    isb();
}

/// Perform the actual page table configuration to ensure 1:1 memory mapping with the desired
/// attributes.
///
/// # Safety
/// A call to this initial MMU setup and configuration should always be called only once and from
/// the main core booting up first only. As long as the MMU is not up and running there is no way
/// to secure access with atmic operations as they require the MMU to not hang the core
fn setup_page_tables() {
    unsafe {
        // create entries for 1:1 memory mappings up to address 0x3F00_0000
        // configure this memory to be shareable, outer/inner write back, allocate on write
        for i in 0..0x3f0 {
            write_volatile(&mut MMU_CFG.ttlb[i] as *mut u32, (i as u32 * 0x10_0000) | 0x91C0E);
        }

        // create entries for 1:1 memory mappings from 0x3F00_0000 to 0xFF00_0000
        // configure this memory to be shared device memory
        for i in 0x3f0..0xfff {
            write_volatile(&mut MMU_CFG.ttlb[i] as *mut u32, (i as u32 * 0x10_0000) | 0x90C16);  
        }
    }
}

/// Maintain the section attribute within the corresponding TTLB's 
pub fn maintain_sections(addr: *mut u8, section_from: usize, section_count: usize, page_attributes: u32) {
    let section_to = section_from + section_count;
    //info!("maintain section from {} to {} for address {:#x?}", section_from, section_to, addr);

    unsafe {
        for i in section_from..section_to {
            //info!("set section {} - address {:#x?}", i, (i as u32 * 0x10_0000));
            write_volatile(&mut MMU_CFG.ttlb[i] as *mut u32, (i as u32 * 0x10_0000) | (page_attributes & 0x000F_FFFF));
        }
        //info!("ttlb maintained");
        // no invalidate the TTLB to take effekt
        llvm_asm!(
            "mcr        p15, 0, $0, c7, c5, 0   // invalidate instruction cache
             mcr		p15, 0, $0, c8, c7, 0	// invalidate unified TLB
             dsb
             isb
             mcr		p15, 0, $0, c8, c3, 0	// invalidate entire inner sharable TLB
             mcr		p15, 0, $0, c8, c5, 0	// invalidate instruction TLB
             mcr		p15, 0, $0, c8, c6, 0   // invalidate data TLB
             dsb
             isb"::"r"(0)
        );
    }
}
