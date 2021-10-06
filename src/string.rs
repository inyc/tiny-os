pub fn mem_set(dst: *mut u64, c: i32, n: u64) -> *mut u64 {
    let cdst = dst as *mut u8;
    for i in 0..n {
        unsafe {
            *cdst.add(i as usize) = c as u8;
        }
    }

    dst
}
