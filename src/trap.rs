use crate::mem_layout::{TRAMPOLINE, TRAP_FRAME};
use crate::plic::plic_intr;
use crate::proc::{my_proc, yield_cpu, ProcState};
use crate::riscv::{
    intr_off, make_satp, rsatp, rscause, rsepc, rsip, rsstatus, rstval, rtp, wsepc, wsip, wsstatus,
    wstvec, PAGE_SIZE, SSTATUS_SIE, SSTATUS_SPIE, SSTATUS_SPP,
};
use core::fmt::Write;
use core::mem::transmute;

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
                let p = my_proc();
                if !p.is_null() {
                    // improve?
                    unsafe {
                        if let ProcState::Running = (*p).state {
                            yield_cpu();
                        }
                    }
                }

                // acknowledge the software interrupt by clearing
                // the SSIP bit in sip.
                wsip(rsip() & !2);
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
    let sstatus = rsstatus();
    let scause = rscause();

    if sstatus & SSTATUS_SPP != 0 {
        panicc!("user_trap: not from u mode");
    }

    wstvec(kernel_vec as u64);

    let p = my_proc();
    unsafe {
        (*(*p).trap_frame).epc = rsepc();
    }

    let intr = match scause & 0x8000_0000_0000_0000 {
        0 => false,
        _ => true,
    };

    if intr {
        match scause & 0xff {
            1 => {
                yield_cpu();
                wsip(rsip() & !2);
            }
            _ => {
                println!("scause 0x{:x}", scause);
                println!("sepc=0x{:x} stval=0x{:x}", rsepc(), rstval());
                panicc!("user_trap");
            }
        }
    } else {
        match scause & 0xff {
            8 => {
                // syscall
                print!("/");
                unsafe {
                    (*(*p).trap_frame).epc += 4;
                }
            }
            12 => {
                panicc!("instruction page fault");
            }
            15 => {
                // AMO atomic mem operation, riscv-sepc p52
                panicc!("store/AMO page fault");
            }
            _ => {
                println!("scause 0x{:x}", scause);
                println!("sepc=0x{:x} stval=0x{:x}", rsepc(), rstval());
                panicc!("user_trap");
            }
        }
    }

    user_trap_ret();
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
        func(TRAP_FRAME, satp);
    }
}
