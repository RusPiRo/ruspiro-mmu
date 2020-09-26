# Changelog

## :apple: v0.1.0

This is the initial release providing the core functionality to properly configure and activate the MMU. The initial configuration enables a 1:1 (physical to virtual) memory mapping for the lower memory reagion addressed by *TTBR0*. As the Raspberry PI uses a memory split between ARM and GPU the maintenance require the memory address and size the GPU memory starts. The memroy below this region is maintained as normal memory with active caching. The memroy region containing the memory mapped IO registered is maintained as device memory without caching.

The memory region used by the GPU is not maintained as this should never being accessed from the ARM side.

- ### :bulb: Features

  - Initialize the MMU for EL2 and EL1
  - maintain initial 1:1 mapping in TTBR0
  - maintain initial root entry in TTBR1 if executed in EL2
  - provide memory mapping function maintain virtual memory mapping in TTBR1 on 2MB block size level
  - provide function to align a given size or memory address to a *page size*
  - return the actual used *page size*
