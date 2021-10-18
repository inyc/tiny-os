
pub const SSTATUS_SIE:u64=1<<1;
pub const SSTATUS_SPP:u64=1<<8; // s mode - 1, u mode - 0

pub fn rsstatus()->u64{
    let x:u64;
    unsafe{
        asm!("csrr {}, sstatus", out(reg) x);
    }
    x
}

pub fn rscause()->u64{
    let x:u64;
    unsafe{
        asm!("csrr {}, scause", out(reg) x);
    }
    x
}


pub fn rsepc()->u64{
    let x:u64;
    unsafe{
        asm!("csrr {}, sepc", out(reg) x);
    }
    x
}

pub fn wstvec(x:u64){
    unsafe{
        asm!("csrw stvec, {}", in(reg) x);
    }
}

// set satp MODE field, where 8 means Sv39
pub const SATP_SV39: u64 = 8 << 60;

pub const fn make_satp(page_table: u64) -> u64 {
    SATP_SV39 | (page_table >> 12)
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

pub const MAX_VA: u64 = 1 << (9 + 9 + 9 + 12 - 1);

pub const PAGE_SIZE: u64 = 4096;
const PAGE_SHIFT: u32 = 12;

pub type Pte = u64;
pub type PageTable = *mut u64;

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
