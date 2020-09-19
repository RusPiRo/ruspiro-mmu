# RusPiRo MMU API

The crate provides the API to configure and maintain the Raspberry Pi Memory Management Unit (MMU). It allows to maintain the EL2 and EL1 configuration settings.

[![Travis-CI Status](https://api.travis-ci.com/RusPiRo/ruspiro-mmu.svg?branch=release)](https://travis-ci.com/RusPiRo/ruspiro-mmu)
[![Latest Version](https://img.shields.io/crates/v/ruspiro-mmu.svg)](https://crates.io/crates/ruspiro-mmu)
[![Documentation](https://docs.rs/ruspiro-mmu/badge.svg)](https://docs.rs/ruspiro-mmu)
[![License](https://img.shields.io/crates/l/ruspiro-mmu.svg)](https://github.com/RusPiRo/ruspiro-mmu#license)

## Usage

To use this crate simply add the dependency to your ``Cargo.toml`` file:

```toml
[dependencies]
ruspiro-mmu = "0.1.0"
```

The initial setup of the MMU should be called only once during the boot sequence of the Raspberry Pi.

With the MMU configured and active a physical memory region can be mapped to a new virtual one with specific memory attributes, different from the initial settings like so:

```rust
use ruspiro_mmu::*;

fn main() {
    // just an arbitrary address for demonstration purposes
    let phys_address = 0xDEADBEEF as *mut u8;
    // the virtual address is of type *mut u8
    let virtual_address = unsafe {
        map_memory(phys_address, 1024,
            ( TTLB_BLOCKPAGE::AF::SET
            | TTLB_BLOCKPAGE::SH::INNER
            | TTLB_BLOCKPAGE::MEMATTR::MAIR3
            | TTLB_BLOCKPAGE::TYPE::BLOCK
            ).raw_value()
        )
    };
}
```

Please note that the current virtual memory mapping is implemented on *block level* only. This means the smallest mapped memory region is 2MB in size regardless of the size given to the `map_memory` function. Therefore the memory attributes passed to the mapping requires to be a `BLOCK` entry. Passing the direct TTLB flags to the memory map function is error prone and will be replaced with proper pre-defined constants to reflect the memeory attribute settings and combinations that are useful.

## License

Licensed under Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0) or MIT ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)) at your choice.