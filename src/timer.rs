use crate::mem_layout::{clint_mtime, clint_mtimecmp};
use crate::param::NCPU;
use crate::riscv::{
    rmhartid, rmie, rmstatus, wmie, wmscratch, wmstatus, wmtvec, MIE_MTIE, MSTATUS_MIE,
};

extern "C" {
    fn timer_vec();
}

static mut TIMER_SCRATCH: [[u64; 5]; NCPU as usize] = [[0; 5]; NCPU as usize];

#[no_mangle]
pub extern "C" fn timer_init() {
    let hart = rmhartid();

    let addr = clint_mtimecmp(hart) as *mut u64;
    let interval = 10_000_000;
    unsafe {
        *addr = interval + *(clint_mtime() as *const u64);

        TIMER_SCRATCH[hart as usize][3] = clint_mtimecmp(hart);
        TIMER_SCRATCH[hart as usize][4] = interval;
        wmscratch(&TIMER_SCRATCH[hart as usize][0] as *const u64 as u64);
    }

    wmtvec(timer_vec as u64);
    wmstatus(rmstatus() | MSTATUS_MIE);
    wmie(rmie() | MIE_MTIE);

    // unsafe{
    //     asm!("mret");
    // }
}
