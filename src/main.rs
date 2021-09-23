// prevent the std crate from being automatically added into scope. It does three things:
// Prevents std from being added to the extern prelude.
// Uses core::prelude::v1 in the standard library prelude instead of std::prelude::v1.
// Injects the core crate into the crate root instead of std, and pulls in all macros exported from core in the macro_use prelude.
#![no_std]
#![no_main]
#![feature(
    panic_info_message,
    llvm_asm,
    global_asm,
    alloc_error_handler,
    alloc_prelude,
    allocator_api
)]

use core::fmt::Write;

extern crate alloc;
use alloc::prelude::v1::*;
use alloc::vec;

extern "C" {
    static TEXT_START: usize;
    static TEXT_END: usize;
    static DATA_START: usize;
    static DATA_END: usize;
    static RODATA_START: usize;
    static RODATA_END: usize;
    static BSS_START: usize;
    static BSS_END: usize;
    static KERNEL_STACK_START: usize;
    static KERNEL_STACK_END: usize;
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
    static mut KERNEL_TABLE: usize;
}

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {
        let _ = write!(crate::uart::Uart::new(0x1000_0000),$($args)+);
    };
}

#[macro_export]
macro_rules! println {
    () => {
        print!("\n")
    };

    ($fmt:expr) => {
        print!(concat!($fmt, "\n"))
    };

    ($fmt:expr, $($args:tt)+) => {
        print!(concat!($fmt, "\n"), $($args)+)
    };
}

// #[no_mangle]
// extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("Aborting: ");
    if let Some(p) = info.location() {
        println!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap()
        );
    } else {
        println!("no info available");
    }
    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            llvm_asm!("wfi"::::"volatile");
        }
    }
}

fn id_map_range(page_table: &mut page::Table, start: usize, end: usize, bits: i64) {
    let mut ptr = page::page_round_down(start);
    let kb_pages_num = (page::page_round_up(end) - start) / page::PAGE_SIZE;

    for _ in 0..kb_pages_num {
        page::map(page_table, ptr, ptr, bits, 0);
        ptr += page::PAGE_SIZE;
    }
}

extern "C" {
    fn switch_to_user(frame: usize) -> !;
}
fn rust_switch_to_user(frame: usize) -> ! {
    unsafe {
        switch_to_user(frame);
    }
}
// ///////////////////////////////////
// / ENTRY POINT
// ///////////////////////////////////
#[no_mangle]
extern "C" fn kinit() {
    uart::Uart::new(0x1000_0000).init();
    page::init();
    kmem::init();
    process::init();
    // We lower the threshold wall so our interrupts can jump over it.
    // Any priority > 0 will be able to be "heard"
    plic::set_threshold(0);
    // VIRTIO = [1..8]
    // UART0 = 10
    // PCIE = [32..35]
    // Enable PLIC interrupts.
    for i in 1..=10 {
        plic::enable(i);
        plic::set_priority(i, 1);
    }
    // Set up virtio. This requires a working heap and page-grained allocator.
    virtio::probe();
    // This just tests the block device. We know that it connects backwards (8, 7, ..., 1).
    let buffer = kmem::kmalloc(1024);
    // Offset 1024 is the first block, which is the superblock. In the minix 3 file system, the first
    // block is the "boot block", which in our case will be 0.
    block::read(8, buffer, 512, 0);
    let mut i = 0;
    loop {
        if i > 100_000_000 {
            break;
        }
        i += 1;
    }
    println!("Test hdd.dsk:");
    unsafe {
        print!("  ");
        for i in 0..16 {
            print!("{:02x} ", buffer.add(i).read());
        }
        println!();
        print!("  ");
        for i in 0..16 {
            print!("{:02x} ", buffer.add(16 + i).read());
        }
        println!();
        print!("  ");
        for i in 0..16 {
            print!("{:02x} ", buffer.add(32 + i).read());
        }
        println!();
        print!("  ");
        for i in 0..16 {
            print!("{:02x} ", buffer.add(48 + i).read());
        }
        println!();
        buffer.add(0).write(0xaa);
        buffer.add(1).write(0xbb);
        buffer.add(2).write(0x7a);
    }
    block::write(8, buffer, 512, 0);
    // Free the testing buffer.
    kmem::kfree(buffer);
    // We schedule the next context switch using a multiplier of 1
    trap::schedule_next_context_switch(1);
    let frame_addr=sched::schedule();
    rust_switch_to_user(frame_addr);
    // switch_to_user will not return, so we should never get here
}

#[no_mangle]
extern "C" fn kinit_hart(hartid: usize) {
    // All non-0 harts initialize here.
    unsafe {
        // We have to store the kernel's table. The tables will be moved
        // back and forth between the kernel's table and user
        // applicatons' tables.

        // cpu::mscratch_write((&mut cpu::KERNEL_TRAP_FRAME[hartid] as *mut cpu::TrapFrame) as usize);

        // Copy the same mscratch over to the supervisor version of the
        // same register.

        // cpu::sscratch_write(cpu::mscratch_read());
        // cpu::KERNEL_TRAP_FRAME[hartid].hartid = hartid;

        // We can't do the following until zalloc() is locked, but we
        // don't have locks, yet :( cpu::KERNEL_TRAP_FRAME[hartid].satp
        // = cpu::KERNEL_TRAP_FRAME[0].satp;
        // cpu::KERNEL_TRAP_FRAME[hartid].trap_stack = page::zalloc(1);
    }
}

// I think now it's not called anymore
#[no_mangle]
extern "C" fn kmain() {
    // kmain() starts in supervisor mode. So, we should have the trap
    // vector setup and the MMU turned on when we get here.

    // We initialized my_uart in machine mode under kinit for debugging
    // prints, but this just grabs a pointer to it.
    let mut my_uart = uart::Uart::new(0x1000_0000);
    // Create a new scope so that we can test the global allocator and
    // deallocator
    {
        // We have the global allocator, so let's see if that works!
        let k = Box::<u32>::new(100);
        println!("Boxed value = {}", *k);
        // The following comes from the Rust documentation:
        // some bytes, in a vector
        let sparkle_heart = vec![240, 159, 146, 150];
        // We know these bytes are valid, so we'll use `unwrap()`.
        // This will MOVE the vector.

        // fuck it
        // let sparkle_heart = String::from_utf8(sparkle_heart).unwrap();
        // println!("String = {}", sparkle_heart);

        println!("\n\nAllocations of a box, vector, and string");
        kmem::print_table();
    }
    println!("\n\nEverything should now be free:");
    kmem::print_table();

    unsafe {
        // Set the next machine timer to fire.
        let mtimecmp = 0x0200_4000 as *mut u64;
        let mtime = 0x0200_bff8 as *const u64;
        // The frequency given by QEMU is 10_000_000 Hz, so this sets
        // the next interrupt to fire one second from now.
        mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);

        // Let's cause a page fault and see what happens. This should trap
        // to m_trap under trap.rs
        let v = 0x0 as *mut u64;
        v.write_volatile(0);
    }
    // If we get here, the Box, vec, and String should all be freed since
    // they go out of scope. This calls their "Drop" trait.

    // Let's set up the interrupt system via the PLIC. We have to set the threshold to something
    // that won't mask all interrupts.
    println!("Setting up interrupts and PLIC...");
    // We lower the threshold wall so our interrupts can jump over it.
    plic::set_threshold(0);
    // VIRTIO = [1..8]
    // UART0 = 10
    // PCIE = [32..35]
    // Enable the UART interrupt.
    plic::enable(10);
    plic::set_priority(10, 1);

    println!("UART interrupts have been enabled and are awaiting your command");
}

mod assembly;
mod block;
mod cpu;
mod kmem;
mod page;
mod plic;
mod process;
mod rng;
mod sched;
mod syscall;
mod trap;
mod uart;
mod virtio;
