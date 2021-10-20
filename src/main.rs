// prevent the std crate from being automatically added into scope. It does three things:
// Prevents std from being added to the extern prelude.
// Uses core::prelude::v1 in the standard library prelude instead of std::prelude::v1.
// Injects the core crate into the crate root instead of std, and pulls in all macros exported from core in the macro_use prelude.
#![no_std]
#![no_main]
#![feature(
    panic_info_message,
    asm,
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

#[macro_export]
macro_rules! panicc {
    ($fmt:expr) => {
        print!("panic: ");
        println!($fmt);
        loop {}
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
            asm!("wfi");
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
    println!("in kinit, s mode");
plic::set_threshold(0);
    // VIRTIO = [1..8]
    // UART0 = 10
    // PCIE = [32..35]
    // Enable PLIC interrupts.
    // for i in 1..=10 {
    //     plic::enable(i);
    //     plic::set_priority(i, 1);
    // }

    //     let mut u=uart::Uart::new(0x1000_0000);
    // loop{

    //     if let Some(x)=u.get(){
    //         print!("{}",x as char);
    //     }
    // }

    plic::enable(10);
    plic::set_priority(10, 1);

unsafe{
            // let v = 0x0 as *mut u64;
        // v.write_volatile(0);
}

    loop {
        // unsafe {
        //     asm!("wfi");
        // }
    }

    // test
    uart::Uart::new(0x1000_0000).init();
    // page::init();

    // kmem::init();
    kalloc::km_init(); // set kmem.free_list
    vm::kvm_init(); // set kernel page table
    vm::kvm_init_hart(); // write satp
    proc::proc_init(); // set proc.kstack
    trap::trap_init_hart(); // set stvec

    let val: u64;
    unsafe {
        asm!("csrr {},stvec",out(reg) val);
    }
    println!("0x{:x}", val);
    println!("0x{:x}", trap::kernel_vec as u64);
    unsafe {
        println!("0x{:x}", TEXT_END);
        println!("0x{:x}", RODATA_END);
        println!("0x{:x}", BSS_END);
        println!("0x{:x}", HEAP_START);
        // trap::kernel_vec();
    }

    // process::init();

    // We lower the threshold wall so our interrupts can jump over it.
    // Any priority > 0 will be able to be "heard"
    // plic::set_threshold(0);
    // VIRTIO = [1..8]
    // UART0 = 10
    // PCIE = [32..35]
    // Enable PLIC interrupts.
    // for i in 1..=10 {
    //     plic::enable(i);
    //     plic::set_priority(i, 1);
    // }

    //     let mut u=uart::Uart::new(0x1000_0000);
    // loop{

    //     if let Some(x)=u.get(){
    //         print!("{}",x as char);
    //     }
    // }

    // plic::enable(10);
    // plic::set_priority(10, 1);
    plic::plic_init_hart();

    unsafe{

        let v = 0x0 as *mut u64;
        v.write_volatile(0);
    }   

    println!("ok");

    loop {
        unsafe {
            asm!("wfi");
        }
    }

    // Set up virtio. This requires a working heap and page-grained allocator.
    virtio::probe();

    // This just tests the block device. We know that it connects backwards (8, 7, ..., 1).
    // let buffer = kmem::kmalloc(1024);
    let buffer = kalloc::kalloc() as *mut u8;

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
    // kmem::kfree(buffer);
    kalloc::kfree(buffer as *mut u64);
    println!("ok");
    loop {}

    // Map heap allocations
    let root_ptr = kmem::get_page_table();
    let root_u = root_ptr as usize;
    let mut root = unsafe { root_ptr.as_mut().unwrap() };
    let kheap_head = kmem::get_head() as usize;
    let total_pages = kmem::get_num_allocations();
    println!();
    println!();
    unsafe {
        println!("TEXT:   0x{:x} -> 0x{:x}", TEXT_START, TEXT_END);
        println!("RODATA: 0x{:x} -> 0x{:x}", RODATA_START, RODATA_END);
        println!("DATA:   0x{:x} -> 0x{:x}", DATA_START, DATA_END);
        println!("BSS:    0x{:x} -> 0x{:x}", BSS_START, BSS_END);
        println!(
            "STACK:  0x{:x} -> 0x{:x}",
            KERNEL_STACK_START, KERNEL_STACK_END
        );
        println!(
            "HEAP:   0x{:x} -> 0x{:x}",
            kheap_head,
            kheap_head + total_pages * page::PAGE_SIZE
        );
    }
    id_map_range(
        &mut root,
        kheap_head,
        kheap_head + total_pages * page::PAGE_SIZE,
        page::EntryBits::ReadWrite.val(),
    );
    // Using statics is inherently unsafe.
    unsafe {
        // Map heap descriptors
        let num_pages = HEAP_SIZE / page::PAGE_SIZE;
        id_map_range(
            &mut root,
            HEAP_START,
            HEAP_START + num_pages,
            page::EntryBits::ReadWrite.val(),
        );
        // Map executable section
        id_map_range(
            &mut root,
            TEXT_START,
            TEXT_END,
            page::EntryBits::ReadExecute.val(),
        );
        // Map rodata section
        // We put the ROdata section into the text section, so they can
        // potentially overlap however, we only care that it's read
        // only.
        id_map_range(
            &mut root,
            RODATA_START,
            RODATA_END,
            page::EntryBits::ReadExecute.val(),
        );
        // Map data section
        id_map_range(
            &mut root,
            DATA_START,
            DATA_END,
            page::EntryBits::ReadWrite.val(),
        );
        // Map bss section
        id_map_range(
            &mut root,
            BSS_START,
            BSS_END,
            page::EntryBits::ReadWrite.val(),
        );
        // Map kernel stack
        id_map_range(
            &mut root,
            KERNEL_STACK_START,
            KERNEL_STACK_END,
            page::EntryBits::ReadWrite.val(),
        );
    }

    // UART
    id_map_range(
        &mut root,
        0x1000_0000,
        0x1000_0100,
        page::EntryBits::ReadWrite.val(),
    );

    // CLINT
    //  -> MSIP
    id_map_range(
        &mut root,
        0x0200_0000,
        0x0200_ffff,
        page::EntryBits::ReadWrite.val(),
    );
    // PLIC
    id_map_range(
        &mut root,
        0x0c00_0000,
        0x0c00_2000,
        page::EntryBits::ReadWrite.val(),
    );
    id_map_range(
        &mut root,
        0x0c20_0000,
        0x0c20_8000,
        page::EntryBits::ReadWrite.val(),
    );
    // When we return from here, we'll go back to boot.S and switch into
    // supervisor mode We will return the SATP register to be written when
    // we return. root_u is the root page table's address. When stored into
    // the SATP register, this is divided by 4 KiB (right shift by 12 bits).
    // We enable the MMU by setting mode 8. Bits 63, 62, 61, 60 determine
    // the mode.
    // 0 = Bare (no translation)
    // 8 = Sv39
    // 9 = Sv48
    // build_satp has these parameters: mode, asid, page table address.
    let satp_value = cpu::build_satp(cpu::SatpMode::Sv39, 0, root_u);
    unsafe {
        // We have to store the kernel's table. The tables will be moved
        // back and forth between the kernel's table and user
        // applicatons' tables. Note that we're writing the physical address
        // of the trap frame.
        cpu::mscratch_write((&mut cpu::KERNEL_TRAP_FRAME[0] as *mut cpu::TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());
        cpu::KERNEL_TRAP_FRAME[0].satp = satp_value;
        // Move the stack pointer to the very bottom. The stack is
        // actually in a non-mapped page. The stack is decrement-before
        // push and increment after pop. Therefore, the stack will be
        // allocated (decremented) before it is stored.
        // cpu::KERNEL_TRAP_FRAME[0].trap_stack =
        // 	page::zalloc(1).add(page::PAGE_SIZE);
        // id_map_range(
        //              &mut root,
        //              cpu::KERNEL_TRAP_FRAME[0].trap_stack
        //                                       .sub(page::PAGE_SIZE,)
        //              as usize,
        //              cpu::KERNEL_TRAP_FRAME[0].trap_stack as usize,
        //              page::EntryBits::ReadWrite.val(),
        // );
        // The trap frame itself is stored in the mscratch register.
        id_map_range(
            &mut root,
            cpu::mscratch_read(),
            cpu::mscratch_read() + core::mem::size_of::<cpu::TrapFrame>(),
            page::EntryBits::ReadWrite.val(),
        );
        page::print_page_allocations();
        // let p = cpu::KERNEL_TRAP_FRAME[0].trap_stack as usize - 1;
        // let m = page::walk(&root, p).unwrap_or(0);
        // println!("Walk 0x{:x} = 0x{:x}", p, m);
    }
    // The following shows how we're going to walk to translate a virtual
    // address into a physical address. We will use this whenever a user
    // space application requires services. Since the user space application
    // only knows virtual addresses, we have to translate silently behind
    // the scenes.
    println!("Setting 0x{:x}", satp_value);
    println!("Scratch reg = 0x{:x}", cpu::mscratch_read());

    cpu::satp_write(satp_value);
    cpu::satp_fence_asid(0);

    // We schedule the next context switch using a multiplier of 1
    // trap::schedule_next_context_switch(1);
    // let frame_addr=sched::schedule();
    // rust_switch_to_user(frame_addr);
    // switch_to_user will not return, so we should never get here
}

#[no_mangle]
extern "C" fn kinit_hart(hartid: usize) {
    // All non-0 harts initialize here.
    unsafe {
        // We have to store the kernel's table. The tables will be moved
        // back and forth between the kernel's table and user
        // applicatons' tables.

        cpu::mscratch_write((&mut cpu::KERNEL_TRAP_FRAME[hartid] as *mut cpu::TrapFrame) as usize);

        // Copy the same mscratch over to the supervisor version of the
        // same register.

        cpu::sscratch_write(cpu::mscratch_read());
        cpu::KERNEL_TRAP_FRAME[hartid].hartid = hartid;

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

    println!();
    println!("now in kmain (supervisor mode)");

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
        // let v = 0x0 as *mut u64;
        // v.write_volatile(0);
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

    // trap::schedule_next_context_switch(1);
    // let frame_addr=sched::schedule();
    // rust_switch_to_user(frame_addr);

    println!("kmain over");
    loop {}

    // println!("UART interrupts have been enabled and are awaiting your command");
}

mod assembly;
mod block;
mod cpu;
mod kalloc;
mod kmem;
mod mem_layout;
mod page;
mod param;
mod plic;
mod proc;
mod process;
mod riscv;
mod rng;
mod sched;
mod string;
mod syscall;
mod trap;
mod uart;
mod virtio;
mod vm;
