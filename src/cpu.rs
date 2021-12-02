pub fn w_medeleg(x: u64) {
    unsafe {
        asm!("csrw medeleg, {}",in(reg) x);
    }
}

pub fn w_mideleg(x: u64) {
    unsafe {
        asm!("csrw mideleg, {}",in(reg) x);
    }
}

pub fn mhartid_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr  {}, mhartid",out(reg) rval);
        rval
    }
}

pub fn mie_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr  {}, mir",out(reg) rval);
        rval
    }
}

pub fn mie_write(val: usize) {
    unsafe {
        asm!("csrw  mie, {}",in(reg) val);
    }
}

pub fn mstatus_write(val: usize) {
    unsafe {
        asm!("csrw  mstatus, {}",in(reg) val);
    }
}

pub fn mstatus_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr  {}, mstatus",out(reg) rval);
        rval
    }
}

pub fn stvec_write(val: usize) {
    unsafe {
        asm!("csrw  stvec, {}",in(reg) val);
    }
}

pub fn stvec_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr  {}, stvec",out(reg) rval);
        rval
    }
}

pub fn mscratch_write(val: usize) {
    unsafe {
        asm!("csrw  mscratch, {}",in(reg) val);
    }
}

pub fn mscratch_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr  {}, mscratch",out(reg) rval);
        rval
    }
}

pub fn mscratch_swap(to: usize) -> usize {
    unsafe {
        let from;
        asm!("csrrw {}, mscratch, {}", out(reg) from, in(reg) to);
        from
    }
}

pub fn sscratch_write(val: usize) {
    unsafe {
        asm!("csrw  sscratch, {}", in(reg) val);
    }
}

pub fn sscratch_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr	{}, sscratch", out(reg) rval);
        rval
    }
}

pub fn sscratch_swap(to: usize) -> usize {
    unsafe {
        let from;
        asm!("csrrw	{}, sscratch, {}", out(reg) from, in(reg) to);
        from
    }
}

pub fn mepc_write(val: usize) {
    unsafe {
        asm!("csrw mepc, {}", in(reg) val);
    }
}

pub fn mepc_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, mepc", out(reg) rval);
        rval
    }
}

pub fn sepc_write(val: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) val);
    }
}

pub fn sepc_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sepc", out(reg) rval);
        rval
    }
}

pub fn satp_write(val: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) val);
    }
}

pub fn satp_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, satp", out(reg) rval);
        rval
    }
}

/// Take a hammer to the page tables and synchronize
/// all of them. This essentially flushes the entire
/// TLB.
pub fn satp_fence(vaddr: usize, asid: usize) {
    unsafe {
        asm!("sfence.vma {}, {}", in(reg) vaddr, in(reg) asid);
    }
}

/// Synchronize based on the address space identifier
/// This allows us to fence a particular process rather
/// than the entire TLB.
/// The RISC-V documentation calls this a TLB flush +.
/// Since there are other memory routines involved, they
/// didn't call it a TLB flush, but it is much like
/// Intel/AMD's invtlb [] instruction.
pub fn satp_fence_asid(asid: usize) {
    unsafe {
        asm!("sfence.vma zero, {}", in(reg) asid);
    }
}
