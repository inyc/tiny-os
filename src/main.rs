// prevent the std crate from being automatically added into scope. It does three things:
// Prevents std from being added to the extern prelude.
// Uses core::prelude::v1 in the standard library prelude instead of std::prelude::v1.
// Injects the core crate into the crate root instead of std, and pulls in all macros exported from core in the macro_use prelude.
#![no_std]
#![no_main]
#![feature(panic_info_message, asm, global_asm)]

use core::fmt::Write;

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
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn kinit() {
    println!("in kinit, s mode");

    uart::Uart::new(mem_layout::UART as usize).init();

    kalloc::km_init(); // set kmem.free_list
    vm::kvm_init(); // set kernel page table
    vm::kvm_init_hart(); // write satp
    proc::proc_init(); // set proc.kstack
    trap::trap_init_hart(); // set stvec
    plic::plic_init(); // set irq priority
    plic::plic_init_hart(); // enable intr and set hart's priority
    virtio_disk::virtio_disk_init(); // intialize the device
    block_cache::binit(); // set the linked list of buffers

    riscv::wsstatus(riscv::SSTATUS_SIE);

    fs::fs_init(param::ROOT_DEV); // main() not call it in xv6, since need sleep

    let inode = fs::iget(1, 1);
    let mut b = block_cache::Buf::new();
    fs::readi(inode, 0, &mut b.data as *mut u8 as u64, 0, 40);

    println!("read ok");
    for i in 0..36 {
        print!("{}", b.data[i] as char);
    }
    println!("");

    proc::user_init(); // set first proc

    println!("init ok");

    proc::scheduler();

    // let mut b = block_cache::Buf {
    //     data: [0; fs::BLOCK_SIZE as usize],
    // };
    // b.data[0] = 1;
    // virtio_disk::virtio_disk_rw(&mut b, 0);
    // let mut ii = 0;
    // loop {
    //     if ii > 1_000 {
    //         break;
    //     }
    //     ii += 1;
    // }

    // println!("read ok");
    // for i in 0..16 {
    //     print!("{:02x} ", b.data[i]);
    // }
    // println!("");
    // for i in 0..16 {
    //     print!("{:02x} ", b.data[i + 16]);
    // }
    // println!("");
    // for i in 0..16 {
    //     print!("{:02x} ", b.data[i + 32]);
    // }
    // println!("");
    // for i in 0..16 {
    //     print!("{:02x} ", b.data[i + 48]);
    // }
    // println!("");
}

mod assembly;
mod block_cache;
mod cpu;
mod fs;
mod kalloc;
mod mem_layout;
mod param;
mod plic;
mod proc;
mod riscv;
mod string;
mod timer;
mod trap;
mod uart;
mod virtio_disk;
mod vm;
