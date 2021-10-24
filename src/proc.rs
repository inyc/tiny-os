use crate::kalloc::kalloc;
use crate::mem_layout::kstack;
use crate::param::{NCPU, NPROC};
use crate::riscv::{intr_get, intr_off, intr_on, rtp, PageTable, PAGE_SIZE, PTE_R, PTE_W};
use crate::vm::kvm_map;
use core::fmt::Write;
use core::ptr::null_mut;

#[repr(C)]
#[derive(Copy, Clone)]
struct Context {
    ra: u64,
    sp: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
}

impl Context {
    const fn new() -> Context {
        Context {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct Cpu {
    proc: *mut Proc,
    context: Context,
}

impl Cpu {
    const fn new() -> Cpu {
        Cpu {
            proc: null_mut(),
            context: Context::new(),
        }
    }
}

static mut CPU: [Cpu; NCPU as usize] = [Cpu::new(); NCPU as usize];

pub fn cpu_id() -> u64 {
    rtp()
}

fn my_cpu() -> *mut Cpu {
    let id = cpu_id();
    let c;
    unsafe {
        c = &mut CPU[id as usize];
    }
    c
}

#[repr(C)]
struct TrapFrame {
    kernel_satp: u64, // 0
    kernel_sp: u64,
    kernel_trap: u64,
    epc: u64,
    kernel_hartid: u64,
    ra: u64,
    sp: u64, // 48
    gp: u64,
    tp: u64,
    t0: u64,
    t1: u64,
    t2: u64,
    s0: u64, // 96
    s1: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64, // 136
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    s2: u64,
    s3: u64, // 184
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64, // 232
    s10: u64,
    s11: u64,
    t3: u64,
    t4: u64,
    t5: u64,
    t6: u64, // 280
}

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
    context: Context,
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
            context: Context::new(),
        }
    }
}

static mut PROC: [Proc; NPROC as usize] = [Proc::new(); NPROC as usize];

static mut NEXT_PID: i32 = 1;

fn my_proc() -> *mut Proc {
    let old = intr_get();
    // if don't disable intr here, then when the process
    // is moved to another cpu, the c will not be mycpu
    intr_off();

    let c = my_cpu();
    let p;
    unsafe {
        p = (*c).proc;
    }

    if old != 0 {
        intr_on();
    }

    p
}

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

extern "C" {
    pub fn switch(old: *const Context, new: *const Context);
}

fn sched() {
    let p = my_proc();

    unsafe {
        if let ProcState::Running = (*p).state {
            panicc!("sched running");
        }
    }

    if intr_get() != 0 {
        panicc!("sched interrupt enabled");
    }

    unsafe{
        switch(&(*p).context,&(*my_cpu()).context);
    }
}

pub fn yield_cpu() {
    let p = my_proc();
    unsafe {
        (*p).state = ProcState::Runnable;
    }
    sched();
}

pub fn scheduler() {
    let c = my_cpu();

    loop {
        // avoid dead lock by intr_on()?

        for i in 0..NPROC as usize {
            unsafe {
                if let ProcState::Runnable = PROC[i].state {
                    PROC[i].state = ProcState::Running;
                    (*c).proc = &mut PROC[i];

                    switch(&(*c).context, &PROC[i].context);

                    (*c).proc = null_mut();
                }
            }
        }
    }
}
