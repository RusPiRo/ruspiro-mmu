/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 *
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/
 #![allow(dead_code)]

//! # MMU Configuration settings
//!
//! The actual implementation of the MMU will rely on the following configuration and settings:
//! While setting up the MMU we configure a 4KB granule size. This means at level 1 each page table entry covers
//! a 1GB memory area and has to point to a level 2 descriptor table. Therefore we will cover here the details starting
//! at level 2.
//!
//! Level 1 and Level 2 covering 1GB / 2MB respectively
//! |Table entry type - Bits |63|62 61|60|59 |58 52|51  48|47                     30|29       12|11          |1 0|
//! |------------------------|--|-----|--|---|-----|------|-------------------------|-----------|------------|---|
//! | Table entry            |NS|AP   |XN|PXN|     | RES0 | Next level table address [47..12]   |            |1 1|
//! | Block entry            |  Block attributes   | RES0 | Output address [47..30] | RES0      | Block attr.|0 1|
//!
//! Level 3 does not allow for further table references, this is the memory page level of the desired granule (4KB)
//! |Table entry type - Bits |63|62 61|60|59 |58 52|51  48|47                     30|29       12|11          |1 0|
//! |------------------------|--|-----|--|---|-----|------|-------------------------|-----------|------------|---|
//! | Page entry             |  Page Attributes    | RES0 | Output address [47..12]             | Page attr. |1 1|
//!
//! The upper and lower block/page attributes are the same on each level of the translation tables. They only differ
//! based on the executed translation stage. The different stages are only relevent in case the translation happens
//! within "user level". This means the first translation stage will map the memory into an intermediate physical
//! address, where the second stage will map this IPA into the real physical address. However, the current RusPiRo MMU
//! setup is configured to only use a one stage translation process always immediately resulting in a physical address.
//!
//!  Upper Attributes (Stage 1)
//! |63     59|58     55| 54 | 53  |52 |
//! |---------|---------|----|-----|---|
//! | ignored | ignored | XN | PXN | C |
//!
//! Bits 63..55 are ignored. The difference here is that bit 63..60 may be used by the MMU implementation of the Chip
//! and bit 58..55 may be used by the actual software
//!
//! Bit  | Description
//! -----|-------------
//!  XN  | eXecute Never bit determining whether the memory region is executable or not.
//!  PXN | Priviliged eXecute Never bit determines whether the memory region is executable in EL1. In EL2/EL3 this bit is RES0
//!  C   | Contigues hint bit indicating that this table entry is one of a contigues sets of entries and might be cached
//!      | together with the other ones
//!
//! Lower Attributes (Stage 1)
//! |11|10  |9  8|7  6|5   |4       2|
//! |--|----|----|----|----|---------|
//! |nG| AF | SH | AP | NS | MemAttr |
//!
//! Bit      | Description
//! ---------|-------------
//!  nG      | not Global bit determines whether this entry is globally valid or only for the current ASID value. This  bit is only valid in EL1 & EL0
//!  AF      | Access Flag bit
//!  SH      | Shareability flag
//!  AP      | data Access Permission bits for AP\[2..1\], AP\[0\] is not defined in the TLB entries
//!  NS      | Non-Secure bit specifies whether the output address is in secure or non-secure address map.
//!  MemAttr | Stage 1 memory attributes - index into MAIR_ELx register

use super::define_tlb_entry;
use ruspiro_arch_aarch64::{RegisterField, RegisterFieldValue};

pub const SECTION_SIZE: usize = 0x20_0000; // 2MB section size
pub const SECTION_MASK: usize = SECTION_SIZE - 1;
pub const SECTION_SHIFT: usize = 21;
pub const PAGE_SIZE: usize = 0x1000; // 4kB page size
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

define_tlb_entry![
    /// # TTLB Table Entry format.
    ///
    /// |Table entry type - Bits |63|62 61|60|59 |58 52|51  48|47                     30|29       12|11         2|1 0|
    /// |------------------------|--|-----|--|---|-----|------|-------------------------|-----------|------------|---|
    /// | Table entry            |NS|AP   |XN|PXN|     | RES0 | Next level table address [47..12]|  |            |1 1|
    pub(crate) TTLB_TABLE {
        /// Flag indicating the table entry is valid or not.
        TYPE OFFSET(0) BITS(2) [
            VALID = 0b11,
            INVALID = 0b00
        ],
        /// Address bits \[47:12\] of the next level table address
        ADDR OFFSET(12) BITS(36),
        /// Priviliged eXecute Never
        PXN OFFSET(59),
        /// eXecute Never
        XN OFFSET(60),
        /// AP flag
        AP OFFSET(61) BITS(2),
        /// Non-Secure access flag
        NS OFFSET(63) [
            SET = 0b1
        ]
    },
    /// # TTLB Block and Page Entry format
    ///
    /// |Table entry type - Bits |\[63     :     52\]|\[51 : 48\]|\[47   :      30\]|\[29 : 12\]|\[11 : 2\]|\[1 : 0\]|
    /// |------------------------|---------------------|------|-------------------------|-----------|------------|---|
    /// | Block entry            |  Block attributes   | RES0 | Output address [47..30] | RES0      | Block attr.|0 1|
    /// | Page entry             |  Page Attributes    | RES0 | Output address [47..12] |           | Page attr. |1 1|
    pub TTLB_BLOCKPAGE {
        TYPE OFFSET(0) BITS(2) [
            BLOCK = 0b01,
            PAGE = 0b11,
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
        //// data Access Permission bits for AP\[2..1\], AP\[0\] is not defined in the TLB entries
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
        /// Output address - bits \[47:12\] are used if this is a page entry.
        /// Output address - bits \[47:30\] are used if this is a block entry.
        ADDR OFFSET(12) BITS(36),
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
