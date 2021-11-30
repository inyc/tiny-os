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

// if not equal, return the first different char in s1
pub fn str_cmp(mut s1: *const u8, mut s2: *const u8, mut size: u32) -> u8 {
    unsafe {
        while size > 0 {
            if (*s1) != (*s2) {
                break;
            }
            size -= 1;
            s1 = s1.add(1);
            s2 = s2.add(1);
        }
    }

    if size == 0 {
        return 0;
    }

    unsafe { (*s1) }
}
