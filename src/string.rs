use core::fmt::Write;

pub fn mem_set(dst: *mut u64, c: i32, n: u64) -> *mut u64 {
    let cdst = dst as *mut u8;
    for i in 0..n {
        unsafe {
            *cdst.add(i as usize) = c as u8;
        }
    }

    dst
}

pub fn mem_copy(dst: *mut u64, src: *const u64, size: u64) {
    let dst_i64 = dst as i64;
    let src_i64 = src as i64;
    // overflow
    if (dst_i64 - src_i64).abs() as u64 <= size - 1 {
        panicc!("mem_copy");
    }

    let cdst = dst as *mut u8;
    let csrc = src as *mut u8;
    for i in 0..size {
        unsafe {
            *cdst.add(i as usize) = *csrc.add(i as usize);
        }
    }
}
