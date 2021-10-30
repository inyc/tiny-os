use crate::kalloc::{kalloc,kfree};
use crate::mem_layout::{kstack, TRAMPOLINE, TRAP_FRAME};
use crate::param::{NCPU, NPROC};
use crate::riscv::{intr_get, intr_off, intr_on, rtp, PageTable, PAGE_SIZE, PTE_R, PTE_W, PTE_X};
use crate::string::mem_set;
use crate::trap::user_trap_ret;
use crate::vm::{kvm_map, map_pages, uvm_free, uvm_unmap,uvm_init};
use core::fmt::Write;
use core::mem::size_of;
use core::ptr::null_mut;

extern "C" {
    // static trampoline: u64;
    fn trampoline();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Context {
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
pub struct TrapFrame {
    pub kernel_satp: u64, // 0
    pub kernel_sp: u64, // 8
    pub kernel_trap: u64, // 16
    pub epc: u64,
    pub kernel_hartid: u64,
    pub ra: u64,
    pub sp: u64, // 48
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64, // 96
    pub s1: u64, // 104
    pub a0: u64, // 112
    pub a1: u64,
    pub a2: u64,
    pub a3: u64, // 136
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64, // 184
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64, // 232
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64, // 280
}

#[derive(Copy, Clone)]
pub enum ProcState {
    Unused,
    Sleeping,
    Runnable,
    Running,
    Zombie,
}

#[derive(Copy, Clone)]
pub struct Proc {
    pub state: ProcState,
    pub parent: *mut Proc,
    pub killed: i32,
    pub pid: i32,

    pub kstack: u64,
    pub size: u64,
    pub page_table: PageTable,
    pub trap_frame: *mut TrapFrame,
    pub context: Context,
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

pub fn my_proc() -> *mut Proc {
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

fn proc_page_table(p: *const Proc) -> PageTable {
    let page_table = kalloc();
    if page_table.is_null() {
        return null_mut();
    }
    mem_set(page_table, 0, PAGE_SIZE as u64);

    unsafe {
        if map_pages(page_table, TRAMPOLINE, PAGE_SIZE, trampoline as u64, PTE_R | PTE_X) != 0 {
            uvm_free(page_table, 0);
            return null_mut();
        }

        if map_pages(
            page_table,
            TRAP_FRAME,
            PAGE_SIZE,
            (*p).trap_frame as u64,
            PTE_R | PTE_W,
        ) != 0
        {
            uvm_unmap(page_table, TRAMPOLINE, 1, 0);
            uvm_free(page_table, 0);
            return null_mut();
        }
    }

    page_table
}

fn proc_free_page_table(page_table:PageTable,size:u64){
    uvm_unmap(page_table,TRAMPOLINE,1,0);
    uvm_unmap(page_table,TRAP_FRAME,1,0);
    uvm_free(page_table,size);
}

fn alloc_pid() -> i32 {
    let pid;
    unsafe {
        pid = NEXT_PID;

        NEXT_PID = NEXT_PID + 1;
    }

    pid
}

// doesn't set p.state
fn alloc_proc() -> *mut Proc {
    let mut i: usize = 0;
    while i < NPROC as usize {
        unsafe {
            if let ProcState::Unused = PROC[i].state {
                break;
            }
        }
        i += 1;
    }

    if i == NPROC as usize {
        return null_mut();
    }

    let p;
    unsafe {
        p = &mut PROC[i];
        (*p).state = ProcState::Runnable;
        (*p).pid = alloc_pid();
        // parent?

        (*p).trap_frame = kalloc() as *mut TrapFrame;
        if (*p).trap_frame.is_null() {
            return null_mut();
        }

        // *((*p).trap_frame as *mut TrapFrame as *mut u8).add(16)=132;
        // println!("287 {}",(*(*p).trap_frame).kernel_trap);

        (*p).page_table = proc_page_table(p);
        if (*p).page_table.is_null(){
            free_proc(p);
            return null_mut();
        }

        mem_set(
            &mut (*p).context as *mut Context as *mut u64,
            0,
            size_of::<Context>() as u64,
        );

        (*p).context.ra = fork_ret as u64;
        (*p).context.sp = (*p).kstack + PAGE_SIZE;
    }

    p
}

fn free_proc(p:*mut Proc){
    unsafe{
        if !(*p).trap_frame.is_null(){
            kfree((*p).trap_frame as *mut u64)
        }

        if !(*p).page_table.is_null(){
            proc_free_page_table((*p).page_table,(*p).size);
        }

        (*p).trap_frame=null_mut();
        (*p).page_table=null_mut();
        (*p).size=0;
        (*p).parent=null_mut();
        (*p).killed=0;
        (*p).pid=0;
        (*p).state=ProcState::Unused;
    }
}

fn first_proc(){
    // let a:*mut u64=0 as *mut u64;
    // unsafe{
    //     (*a)=0;
    // }
    loop{}
}

pub fn user_init(){
    let p=alloc_proc();

    unsafe{
        // !--dangerous--!
        uvm_init((*p).page_table,first_proc as *const u64,PAGE_SIZE);
        (*p).size=PAGE_SIZE;

        
        (*(*p).trap_frame).epc=0;
        (*(*p).trap_frame).sp=PAGE_SIZE;

        (*p).state=ProcState::Runnable;

    }
}

extern "C" {
    fn switch(old: *const Context, new: *const Context);
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

    unsafe {
        switch(&(*p).context, &(*my_cpu()).context);
    }
}

pub fn yield_cpu() {
    let p = my_proc();
    unsafe {
        (*p).state = ProcState::Runnable;
    }
    sched();
}

fn fork_ret() {
    user_trap_ret();
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

            print!(".");
        }
    }
}
