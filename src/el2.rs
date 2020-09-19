/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 *
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/

//! # MMU Exception Level 2
//!

use ruspiro_arch_aarch64::instructions::nop;
use ruspiro_arch_aarch64::register::el2::{hcr_el2, mair_el2, sctlr_el2, tcr_el2, ttbr0_el2};

pub fn enable_mmu(ttlb_base_addr: u64) {
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
    ttbr0_el2::write(ttbr0_el2::BADDR::with_value(ttlb_base_addr));

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

    unsafe {
        llvm_asm!("tlbi  alle2");
    }
}

pub fn disable_mmu() {
    sctlr_el2::write(sctlr_el2::M::DISABLE | sctlr_el2::C::DISABLE | sctlr_el2::I::DISABLE);
    unsafe {
        llvm_asm!("tlbi  alle2");
    }
}
