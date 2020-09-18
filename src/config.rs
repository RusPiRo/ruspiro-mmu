/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 * 
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/
 #![allow(dead_code)]

//! # MMU Configuration settings
//!

use ruspiro_register::{RegisterField, RegisterFieldValue};
use super::define_tlb_entry;

pub const SECTION_SIZE: usize = 0x20_0000; // 2MB section size
pub const SECTION_MASK: usize = SECTION_SIZE - 1;
pub const SECTION_SHIFT: usize = 21;
pub const PAGE_SIZE: usize = 0x1000; // 4kB page size
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

define_tlb_entry![
    /// TTLB Table Entry format
    pub TABLE {
        TYPE OFFSET(0) BITS(2) [
            VALID = 0b11,
            INVALID = 0b00
        ],
        PXN OFFSET(59),
        XN OFFSET(60),
        AP OFFSET(61) BITS(2),
        NS OFFSET(63) [
            SET = 0b1
        ]
    },
    /// TTLB Block Entry format
    pub BLOCK {
        TYPE OFFSET(0) BITS(2) [
            VALID = 0b01,
            INVALID = 0b00
        ],
        /// Stage 1 memory attributes - index into MAIR_ELx register
        MEMATTR OFFSET(2) BITS(3) [
            MAIR0 = 0,
            MAIR1 = 1,
            MAIR2 = 2,
            MAIR3 = 3,
            MAIR4 = 4,
            MAIR5 = 5,
            MAIR6 = 6,
            MAIR7 = 7
        ],
        /// Non-Secure bit specifies whether the output address is in secure or non-secure address map.
        NS OFFSET(5) [
            SET = 0b1
        ],
        //// data Access Permission bits for AP[2..1], AP[0] is not defined in the TLB entries
        AP OFFSET(6) BITS(2),
        /// Shareability flag
        SH OFFSET(8) BITS(2) [
            INNER = 0b11
        ],
        /// Access Flag bit
        AF OFFSET(10) [
            SET = 0b1
        ],
        /// not Global bit determines whether this entry is globally valid or only for the current ASID value. This  
        /// bit is only valid in EL1 & EL0
        NG OFFSET(11),
        /// Contigues hint bit indicating that this table entry is one of a contigues sets of entries and might be 
        /// cached together with the other ones
        C OFFSET(52),
        /// Priviliged eXecute Never bit determines whether the memory region is executable in EL1. In EL2/EL3 this bit 
        /// is RES0
        PXN OFFSET(53),
        /// eXecute Never bit determining whether the memory region is executable or not.
        XN OFFSET(54)
    }
];