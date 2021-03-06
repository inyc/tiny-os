pub const MSTATUS_MIE: u64 = 1 << 3;
pub const MIE_MTIE: u64 = 1 << 7;

pub fn rmhartid() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, mhartid",out(reg) x);
    }
    x
}

pub fn rmstatus() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, mstatus", out(reg) x);
    }
    x
}

pub fn wmstatus(x: u64) {
    unsafe {
        asm!("csrw mstatus, {}", in(reg) x);
    }
}

pub fn wmscratch(x: u64) {
    unsafe {
        asm!("csrw mscratch, {}", in(reg) x);
    }
}

pub fn wmtvec(x: u64) {
    unsafe {
        asm!("csrw mtvec, {}", in(reg) x);
    }
}

pub fn rmie() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, mie", out(reg) x);
    }
    x
}

pub fn wmie(x: u64) {
    unsafe {
        asm!("csrw mie, {}", in(reg) x);
    }
}

pub const SSTATUS_SIE: u64 = 1 << 1;
pub const SSTATUS_SPIE: u64 = 1 << 5;
pub const SSTATUS_SPP: u64 = 1 << 8; // s mode - 1, u mode - 0

pub fn rsstatus() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) x);
    }
    x
}

pub fn wsstatus(x: u64) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) x);
    }
}

pub fn intr_on() {
    wsstatus(rsstatus() | SSTATUS_SIE);
}

pub fn intr_off() {
    wsstatus(rsstatus() & !SSTATUS_SIE);
}

pub fn intr_get() -> u64 {
    rsstatus() & SSTATUS_SIE
}

pub fn rscause() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) x);
    }
    x
}

pub fn rsepc() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, sepc", out(reg) x);
    }
    x
}

pub fn wsepc(x: u64) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) x);
    }
}

pub fn rstval() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, stval", out(reg) x);
    }
    x
}

pub fn wstvec(x: u64) {
    unsafe {
        asm!("csrw stvec, {}", in(reg) x);
    }
}

pub fn rsip() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, sip", out(reg) x);
    }
    x
}

pub fn wsip(x: u64) {
    unsafe {
        asm!("csrw sip, {}", in(reg) x);
    }
}

// set satp MODE field, where 8 means Sv39
pub const SATP_SV39: u64 = 8 << 60;

pub const fn make_satp(page_table: u64) -> u64 {
    SATP_SV39 | (page_table >> 12)
}

pub fn rsatp() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, satp", out(reg) x);
    }
    x
}

pub fn wsatp(x: u64) {
    unsafe {
        asm!("csrw satp, {}", in(reg) x);
    }
}

pub fn sfence_vma() {
    unsafe {
        // rs1=x0 and rs2=x0, orders all reads and writes
        // made to any level of the page tables, for all address spaces
        asm!("sfence.vma zero, zero");
    }
}

pub fn rtp() -> u64 {
    let x: u64;
    unsafe {
        asm!("mv {}, tp", out(reg) x);
    }
    x
}

pub const MAX_VA: u64 = 1 << (9 + 9 + 9 + 12 - 1);

pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u32 = 12;

pub type Pte = u64;
pub type PageTable = *mut u64;

pub const fn pte_flags(pte: Pte) -> u64 {
    pte & 0x3ff
}

// PTE_X:
// whether the CPU may interpret the content of the page
// as instructions and execute them
pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;
pub const PTE_U: u64 = 1 << 4;

pub const fn pa_to_pte(pa: u64) -> u64 {
    (pa >> 12) << 10
}

pub const fn pte_to_pa(pte: Pte) -> u64 {
    (pte >> 10) << 12
}

pub const fn page_round_up(a: u64) -> u64 {
    (a + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub const fn page_round_down(a: u64) -> u64 {
    a & !(PAGE_SIZE - 1)
}

pub const fn vpn(level: i32, va: u64) -> u64 {
    (va >> (12 + 9 * level)) & 0x1ff
}
