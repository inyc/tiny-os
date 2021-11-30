use crate::mem_layout::{plic_sclaim, plic_senable, plic_spriority, PLIC, UART_IRQ, VIRTIO_IRQ};
use crate::proc::cpu_id;
use crate::uart::{uart_intr};
use crate::virtio_disk::virtio_disk_intr;

use core::fmt::Write;

pub fn plic_init() {
    unsafe {
        // set desired IRQ priorities
        *((PLIC + UART_IRQ * 4) as *mut u32) = 1;
        *((PLIC + VIRTIO_IRQ * 4) as *mut u32) = 1;
    }
}

pub fn plic_init_hart() {
    let hart = cpu_id();
    unsafe {
        *(plic_senable(hart) as *mut u32) = (1 << UART_IRQ) | (1 << VIRTIO_IRQ);
        // set this hart's S-mode priority threshold
        *(plic_spriority(hart) as *mut u32) = 0;
    }
}

fn plic_claim() -> Option<u32> {
    let hart = cpu_id();

    let irq: u32;
    unsafe {
        irq = *(plic_sclaim(hart) as *const u32);
    }

    match irq {
        0 => None,
        _ => Some(irq),
    }
}

fn plic_complete(irq: u32) {
    let hart = cpu_id();
    unsafe {
        *(plic_sclaim(hart) as *mut u32) = irq;
    }
}

// called in trap
pub fn plic_intr() {
    if let Some(irq) = plic_claim() {
        match irq as u64 {
            UART_IRQ => {
                uart_intr();
            }
            VIRTIO_IRQ => {
                virtio_disk_intr();
            }
            _ => {
                println!("unexpected interrupt id irq={}", irq);
            }
        }

        plic_complete(irq);
    }
}
