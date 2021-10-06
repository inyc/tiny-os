use crate::mem_layout::PHY_STOP;
use crate::riscv::{page_round_up, PAGE_SIZE};
use core::fmt::Write;
use core::ptr::null_mut;

extern "C" {
    static HEAP_START: u64;
}

struct Run {
    next: *mut Run,
}

struct Kmem {
    free_list: *mut Run,
}

static mut KMEM: Kmem = Kmem {
    free_list: null_mut(),
};

pub fn km_init() {
    unsafe {
        free_range(HEAP_START as *mut u64, PHY_STOP as *mut u64);
    }
}

fn free_range(pa_start: *mut u64, pa_end: *mut u64) {
    let mut p = page_round_up(pa_start as u64) as *mut u8;
    unsafe {
        while p.add(PAGE_SIZE as usize) as *mut u64 <= pa_end {
            kfree(p as *mut u64);
            p = p.add(PAGE_SIZE as usize);
        }
    }
}

pub fn kfree(pa: *mut u64) {
    let pa_u64 = pa as u64;

    unsafe {
        if pa_u64 % PAGE_SIZE != 0 || pa_u64 < HEAP_START || pa_u64 >= PHY_STOP {
            panicc!("kfree");
        }

        let r = pa as *mut Run;
        (*r).next = KMEM.free_list;
        KMEM.free_list = r;
    }
}

pub fn kalloc() -> *mut u64 {
    let r;
    unsafe {
        r = KMEM.free_list;
        if !r.is_null() {
            KMEM.free_list = (*r).next;
        }
    }

    r as *mut u64
}
