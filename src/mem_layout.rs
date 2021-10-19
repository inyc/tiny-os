use crate::riscv::{MAX_VA, PAGE_SIZE};

pub const UART: u64 = 0x1000_0000;
pub const UART_IRQ: u64 = 10;

pub const VIRTIO: u64 = 0x1000_1000;
pub const VIRTIO_IRQ:u64=1;

pub const PLIC: u64 = 0x0c00_0000;

pub const fn plic_senable(hart: u64) -> u64 {
    PLIC + 0x2080 + hart * 0x100
}

pub const fn plic_spriority(hart: u64) -> u64 {
    PLIC + 0x20_1000 + hart * 0x2000
}

pub const KERN_BASE: u64 = 0x8000_0000;
pub const PHY_STOP: u64 = KERN_BASE + 128 * 1024 * 1024;

pub const TRAMPOLINE: u64 = MAX_VA - PAGE_SIZE;

pub const fn kstack(p: u32) -> u64 {
    TRAMPOLINE - (p as u64 + 1) * 2 * PAGE_SIZE
}
