use crate::cpu::TrapFrame;
use core::fmt::Write;

pub fn do_syscall(mepc: usize, frame: *mut TrapFrame) -> usize {
    let syscall_number;
    unsafe {
        // A0 is X10, so it's register number 10.
        syscall_number = (*frame).regs[10];
        // for i in 0..32 {
        //     print!("regs[{:02}] = 0x{:08x}    ", i, (*frame).regs[i]);
        //     if (i+1) % 4 == 0 {
        //         println!();
        //     }
        // }
    }
    match syscall_number {
        0 => {
            // Exit
            // Currently, we cannot kill a process, it runs forever. We will delete
            // the process later and free the resources, but for now, we want to get
            // used to how processes will be scheduled on the CPU.
            mepc + 4
        }
        1 => {
            println!("Test syscall");
            mepc + 4
        }
        _ => {
            println!("Unknown syscall number {}", syscall_number);
            mepc + 4
        }
    }
}

// above is sgmarz code
