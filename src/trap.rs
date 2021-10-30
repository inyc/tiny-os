use crate::cpu::{TrapFrame, CONTEXT_SWITCH_TIME};
use crate::mem_layout::{TRAMPOLINE, TRAP_FRAME};
use crate::plic::{handle_interrupt, plic_intr};
use crate::proc::my_proc;
use crate::riscv::{
    intr_off, make_satp, rsatp, rscause, rsepc, rsstatus, rstval, rtp, wsepc, wsstatus, wstvec,
    PAGE_SIZE, SSTATUS_SIE, SSTATUS_SPIE, SSTATUS_SPP,
};
use crate::rust_switch_to_user;
use crate::sched::schedule;
use crate::syscall::do_syscall;
use crate::uart::Uart;
use core::fmt::Write;
use core::mem::transmute;

#[no_mangle]
/// The m_trap stands for "machine trap". Right now, we are handling
/// all traps at machine mode. In this mode, we can figure out what's
/// going on and send a trap where it needs to be. Remember, in machine
/// mode and in this trap, interrupts are disabled and the MMU is off.
extern "C" fn m_trap(
    epc: usize,
    tval: usize,
    cause: usize,
    hart: usize,
    _status: usize,
    frame: *mut TrapFrame,
) -> usize {
    // We're going to handle all traps in machine mode. RISC-V lets
    // us delegate to supervisor mode, but switching out SATP (virtual memory)
    // gets hairy.
    let is_async = {
        if cause >> 63 & 1 == 1 {
            true
        } else {
            false
        }
    };
    // The cause contains the type of trap (sync, async) as well as the cause
    // number. So, here we narrow down just the cause number.
    let cause_num = cause & 0xfff;
    let mut return_pc = epc;
    if is_async {
        // Asynchronous trap
        match cause_num {
            3 => {
                // Machine software
                println!("Machine software interrupt CPU #{}", hart);
            }
            7 => {
                // This is the context-switch timer.
                // We would typically invoke the scheduler here to pick another
                // process to run.
                // Machine timer
                // println!("CTX");
                let frame = schedule();
                schedule_next_context_switch(1);
                rust_switch_to_user(frame);
            }
            11 => {
                // Machine external (interrupt from Platform Interrupt Controller (PLIC))
                // println!("Machine external interrupt CPU#{}", hart);
                // We will check the next interrupt. If the interrupt isn't available, this will
                // give us None. However, that would mean we got a spurious interrupt, unless we
                // get an interrupt from a non-PLIC source. This is the main reason that the PLIC
                // hardwires the id 0 to 0, so that we can use it as an error case.
                handle_interrupt();
            }
            _ => {
                panic!("Unhandled async trap CPU#{} -> {}\n", hart, cause_num);
            }
        }
    } else {
        // Synchronous trap
        match cause_num {
            2 => {
                // Illegal instruction
                panic!(
                    "Illegal instruction CPU#{} -> 0x{:08x}: 0x{:08x}\n",
                    hart, epc, tval
                );
                // We need while trues here until we have a functioning "delete from scheduler"
                // I use while true because Rust will warn us that it looks stupid.
                // This is what I want so that I remember to remove this and replace
                // them later.
                while true {}
            }
            8 => {
                // Environment (system) call from User mode
                // println!("E-call from User mode! CPU#{} -> 0x{:08x}", hart, epc);
                return_pc = do_syscall(return_pc, frame);
            }
            9 => {
                // Environment (system) call from Supervisor mode
                println!("E-call from Supervisor mode! CPU#{} -> 0x{:08x}", hart, epc);
                return_pc = do_syscall(return_pc, frame);
            }
            11 => {
                // Environment (system) call from Machine mode
                panic!("E-call from Machine mode! CPU#{} -> 0x{:08x}\n", hart, epc);
            }
            // Page faults
            12 => {
                // Instruction page fault
                println!(
                    "Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                // We need while trues here until we have a functioning "delete from scheduler"
                while true {}
                return_pc += 4;
            }
            13 => {
                // Load page fault
                println!(
                    "Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                // We need while trues here until we have a functioning "delete from scheduler"
                while true {}
                return_pc += 4;
            }
            15 => {
                // Store page fault
                println!(
                    "Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}",
                    hart, epc, tval
                );
                // We need while trues here until we have a functioning "delete from scheduler"
                while true {}
                return_pc += 4;
            }
            _ => {
                panic!("Unhandled sync trap CPU#{} -> {}\n", hart, cause_num);
            }
        }
    };
    // Finally, return the updated program counter
    return_pc
}

pub const MMIO_MTIMECMP: *mut u64 = 0x0200_4000usize as *mut u64;
pub const MMIO_MTIME: *const u64 = 0x0200_BFF8 as *const u64;

pub fn schedule_next_context_switch(qm: u16) {
    // This is much too slow for normal operations, but it gives us
    // a visual of what's happening behind the scenes.
    unsafe {
        MMIO_MTIMECMP.write_volatile(
            MMIO_MTIME
                .read_volatile()
                .wrapping_add(CONTEXT_SWITCH_TIME * qm as u64),
        );
    }
}

// the above is sgmarz_code

extern "C" {
    pub fn kernel_vec();
}

pub fn trap_init_hart() {
    wstvec(kernel_vec as u64);
}

#[no_mangle]
extern "C" fn kernel_trap() {
    let sstatus = rsstatus();
    let scause = rscause();
    let sepc = rsepc();

    if sstatus & SSTATUS_SPP == 0 {
        panicc!("kernel trap: not from s mode");
    }
    if sstatus & SSTATUS_SIE != 0 {
        panicc!("kernel trap: interrupts enabled");
    }

    let intr = match scause & 0x8000_0000_0000_0000 {
        0 => false,
        _ => true,
    };

    if intr {
        match scause & 0xff {
            1 => {
                // time interrupt
                print!(".");
                loop{}
            }
            9 => {
                plic_intr();
            }
            _ => {
                println!("scause 0x{:x}", scause);
                println!("sepc=0x{:x} stval=0x{:x}", sepc, rstval());
                panicc!("kernel_trap");
            }
        }
    } else {
        match scause & 0xff {
            5 => {
                panicc!("load access fault");
            }
            13 => {
                panicc!("load page fault");
            }
            _ => {
                println!("scause 0x{:x}", scause);
                println!("sepc=0x{:x} stval=0x{:x}", sepc, rstval());
                panicc!("kernel_trap");
            }
        }
    }
}

extern "C" {
    fn user_vec();
    fn user_ret();
    fn trampoline();
}

fn user_trap() {
    print!("user trap");
    loop{}
}

pub fn user_trap_ret() {
    let p = my_proc();

    intr_off();

    unsafe {
        wstvec(TRAMPOLINE + (user_vec as u64 - trampoline as u64));

        (*(*p).trap_frame).kernel_satp = rsatp();
        (*(*p).trap_frame).kernel_sp = (*p).kstack + PAGE_SIZE;
        (*(*p).trap_frame).kernel_trap = user_trap as u64;
        (*(*p).trap_frame).kernel_hartid = rtp();
    }

    let mut x = rsstatus();
    x &= !SSTATUS_SPP; // 0 - u mode
    x |= SSTATUS_SPIE;
    wsstatus(x);

    unsafe {
        wsepc((*(*p).trap_frame).epc);

        let satp = make_satp((*p).page_table as u64);

        let addr = TRAMPOLINE + (user_ret as u64 - trampoline as u64);
        let func = transmute::<u64, fn(u64, u64)>(addr);
        println!("TRAMPOLINE: 0x{:x}",TRAMPOLINE as u64);
        // println!("p trampoline: 0x{:x}",trampoline as u64);
        // println!("user_vec: 0x{:x}",user_vec as u64);
        // println!("user_ret: 0x{:x}",user_ret as u64);
        let a=TRAMPOLINE + (user_vec as u64 - trampoline as u64);
        println!("user_vec: 0x{:x}",a as u64);
        println!("pa: 0x{:x}", crate::vm::walk_addr((*p).page_table,0x3fffffe004 as u64));
        println!("0x{:x}",TRAP_FRAME as u64);
        // loop{}
        func(TRAP_FRAME, satp);
    }
}
