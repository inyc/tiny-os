use crate::kalloc::kalloc;
use crate::mem_layout::kstack;
use crate::param::NPROC;
use crate::riscv::{PageTable, PAGE_SIZE, PTE_R, PTE_W};
use crate::vm::kvm_map;
use core::fmt::Write;
use core::ptr::null_mut;

#[repr(C)]
struct TrapFrame {}

#[derive(Copy, Clone)]
enum ProcState {
    Unused,
    Sleeping,
    Runnable,
    Running,
    Zombie,
}

#[derive(Copy, Clone)]
struct Proc {
    state: ProcState,
    parent: *mut Proc,
    killed: i32,
    pid: i32,

    kstack: u64,
    size: u64,
    page_table: PageTable,
    trap_frame: *mut TrapFrame,
}

impl Proc {
    const fn new() -> Proc {
        Proc {
            state: ProcState::Unused,
            parent: null_mut(),
            killed: 0,
            pid: 0,

            kstack: 0,
            size: 0,
            page_table: null_mut(),
            trap_frame: null_mut(),
        }
    }
}

static mut PROC: [Proc; NPROC as usize] = [Proc::new(); NPROC as usize];

static mut NEXT_PID: i32 = 1;

pub fn proc_map_stacks(kpg_tbl: PageTable) {
    for i in 0..NPROC {
        let pa = kalloc();
        if pa.is_null() {
            panicc!("kalloc");
        }

        let va = kstack(i);
        kvm_map(kpg_tbl, va, pa as u64, PAGE_SIZE, PTE_R | PTE_W);
    }
}

pub fn proc_init() {
    for i in 0..NPROC {
        unsafe {
            PROC[i as usize].kstack = kstack(i);
        }
    }
}

fn alloc_pid() -> i32 {
    let pid;
    unsafe {
        pid = NEXT_PID;

        NEXT_PID = NEXT_PID + 1;
    }

    pid
}

fn alloc_proc() {}
