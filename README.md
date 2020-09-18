# RusPiRo MMU API

The crate provides the API to configure and maintain the Raspberry Pi Memory Management Unit (MMU). It allows to maintain the EL2 and EL1 configuration settings.

[![Travis-CI Status](https://api.travis-ci.org/RusPiRo/ruspiro-mmu.svg?branch=release)](https://travis-ci.org/RusPiRo/ruspiro-mmu)
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
    let phys_address = 0xDEADBEEF as *mut u8;
    let virtual_address = unsafe {
        map_memory(phys_address, 1024,
            ( BLOCK::AF::SET
            | BLOCK::SH::INNER
            | BLOCK::MEMATTR::MAIR3
            ).raw_value()
        )
    };
}
```

## License

Licensed under Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0) or MIT ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)) at your choice.