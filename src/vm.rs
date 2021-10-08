use crate::kalloc::kalloc;
use crate::mem_layout::{KERN_BASE, PHY_STOP, PLIC, UART, VIRTO};
use crate::proc::proc_map_stacks;
use crate::riscv::{
    pa_to_pte, page_round_down, pte_to_pa, vpn, PageTable, Pte, MAX_VA, PAGE_SIZE, PTE_R, PTE_V,
    PTE_W, PTE_X,
};
use crate::string::mem_set;
use core::fmt::Write;
use core::ptr::null_mut;

extern "C" {
    static TEXT_END: u64;
}

static mut KERNEL_PAGE_TABLE: PageTable = null_mut();

fn kvm_make() -> PageTable {
    let kpg_tbl = kalloc() as PageTable;
    mem_set(kpg_tbl, 0, PAGE_SIZE);

    kvm_map(kpg_tbl, UART, UART, PAGE_SIZE, PTE_R | PTE_W);

    kvm_map(kpg_tbl, VIRTO, VIRTO, PAGE_SIZE, PTE_R | PTE_W);

    kvm_map(kpg_tbl, PLIC, PLIC, 0x400000, PTE_R | PTE_W);

    unsafe {
        // map kernel text executable and read-only
        kvm_map(
            kpg_tbl,
            KERN_BASE,
            KERN_BASE,
            TEXT_END - KERN_BASE,
            PTE_R | PTE_X,
        );

        // map kernel data and the physical RAM
        kvm_map(
            kpg_tbl,
            TEXT_END,
            TEXT_END,
            PHY_STOP - TEXT_END,
            PTE_R | PTE_W,
        );
    }

    proc_map_stacks(kpg_tbl);

    kpg_tbl
}

pub fn kvm_init() {
    unsafe {
        KERNEL_PAGE_TABLE = kvm_make();
    }
}

fn walk(mut page_table: PageTable, va: u64, alloc: i32) -> *mut Pte {
    if va >= MAX_VA {
        panicc!("walk");
    }

    for level in (1..=2).rev() {
        unsafe {
            let mut pte = page_table.add(vpn(level, va) as usize);
            if (*pte) & PTE_V != 0 {
                page_table = pte_to_pa(*pte) as PageTable;
            } else {
                if alloc == 0 {
                    return null_mut();
                }

                page_table = kalloc();
                if page_table.is_null() {
                    return null_mut();
                }

                mem_set(page_table, 0, PAGE_SIZE);
                *pte = pa_to_pte(page_table as u64) | PTE_V;
            }
        }
    }

    unsafe { page_table.add(vpn(0, va) as usize) }
}

pub fn kvm_map(kpgtbl: PageTable, va: u64, pa: u64, size: u64, perm: u64) {
    if map_pages(kpgtbl, va, size, pa, perm) != 0 {
        panicc!("kvm_map");
    }
}

fn map_pages(page_table: PageTable, va: u64, size: u64, mut pa: u64, perm: u64) -> i32 {
    let mut a = page_round_down(va);
    let last = page_round_down(va + size - 1);

    loop {
        let pte = walk(page_table, a, 1);
        if pte.is_null() {
            return -1;
        }

        unsafe {
            if (*pte) & PTE_V != 0 {
                panicc!("remap");
            }
            *pte = pa_to_pte(pa) | perm | PTE_V;
        }

        if a == last {
            break;
        }
        a += PAGE_SIZE;
        pa += PAGE_SIZE;
    }

    0
}
