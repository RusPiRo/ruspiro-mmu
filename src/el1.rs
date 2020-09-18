/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 * 
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/

//! # MMU Exception Level 1
//!

use ruspiro_arch_aarch64::instructions::{isb, nop};
use ruspiro_arch_aarch64::register::el1::{mair_el1, sctlr_el1, tcr_el1, ttbr0_el1, ttbr1_el1};

pub fn enable_mmu(ttbr0_addr: u64, ttbr1_addr: u64) {
    // configure the MAIR (memory attribute) variations we will support
    // those entries are referred to as index in the memeory attributes of the
    // table entries
    mair_el1::write(
        mair_el1::MAIR0::NGNRNE
            | mair_el1::MAIR1::NGNRE
            | mair_el1::MAIR2::GRE
            | mair_el1::MAIR3::NC
            | mair_el1::MAIR4::NORM
            | mair_el1::MAIR5::NOWTIWT
            | mair_el1::MAIR6::NOWTNTIWTNT
            | mair_el1::MAIR7::NOWBTIWBT,
    );

    // set the ttlb base address for the 1:1 translation table configuration
    // of the lower memory region
    //let ttlb_base = unsafe { (&MMU_CFG0.ttlb_lvl1[0] as *const u64) as u64 };
    ttbr0_el1::write(ttbr0_el1::BADDR::with_value(ttbr0_addr));
    ttbr1_el1::write(ttbr1_el1::BADDR::with_value(ttbr1_addr));

    // configure the TTLB attributes, the memory attributes used here has to match the memory attributes
    // used when setting up the translation table entries covering the region the translation table is located at
    // as the lowest granule is 4kB the translation tables should always cover this 4kB to ensure no other dynamic
    // allocated memory may require a different configuration falling into the same 4kB page
    tcr_el1::write(
        tcr_el1::T0SZ::with_value(25)
            | tcr_el1::EPD0::ENABLE
            | tcr_el1::IRGN0::NM_IWB_RA_WA
            | tcr_el1::ORGN0::NM_OWB_RA_WA
            | tcr_el1::SH0::IS
            | tcr_el1::TG0::_4KB
            | tcr_el1::T1SZ::with_value(25) // makes lower address range 0x0 - 0x7F_FFFF_FFFF
            | tcr_el1::EPD1::ENABLE
            | tcr_el1::IRGN1::NM_IWB_RA_WA
            | tcr_el1::ORGN1::NM_OWB_RA_WA
            | tcr_el1::SH1::IS
            | tcr_el1::TG1::_4KB
            | tcr_el1::IPS::_32BITS
            | tcr_el1::TBI0::IGNORE,
    );

    // ensure TCR_EL1 and TTBR0_EL1 changes are seen before MMU is activated
    isb();
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
    // force MMU changes to be seen by the next instruction
    isb();

    unsafe {
        llvm_asm!("tlbi  vmalle1");
    }
}

pub fn disable_mmu() {
    sctlr_el1::write(sctlr_el1::M::DISABLE | sctlr_el1::C::DISABLE | sctlr_el1::I::DISABLE);
    unsafe {
        llvm_asm!("tlbi  vmalle1");
    }
}
