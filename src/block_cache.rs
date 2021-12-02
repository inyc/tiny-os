use crate::fs::BLOCK_SIZE;
use crate::param::NBUF;
use crate::virtio_disk::virtio_disk_rw;
use core::fmt::Write;
use core::ptr::null_mut;

#[derive(Copy, Clone)]
pub struct Buf {
    pub valid: u32,
    pub ref_cnt: u32,
    pub dev: u32,
    pub block_no: u32,
    pub prev: *mut Buf,
    pub next: *mut Buf,
    pub data: [u8; BLOCK_SIZE as usize],
}

impl Buf {
    pub const fn new() -> Self {
        Buf {
            valid: 0,
            ref_cnt: 0,
            dev: 0,
            block_no: 0,
            prev: null_mut(),
            next: null_mut(),
            data: [0; BLOCK_SIZE as usize],
        }
    }
}

struct Bcache {
    head: *mut Buf,
    buf: [Buf; NBUF],
}

static mut BCACHE: Bcache = Bcache {
    head: null_mut(),
    buf: [Buf::new(); NBUF],
};

pub fn binit() {
    unsafe {
        for i in 0..NBUF - 1 {
            BCACHE.buf[i].next = &mut BCACHE.buf[i + 1] as *mut Buf;
        }
        BCACHE.buf[NBUF - 1].next = &mut BCACHE.buf[0] as *mut Buf;

        for i in (1..NBUF).rev() {
            BCACHE.buf[i].prev = &mut BCACHE.buf[i - 1] as *mut Buf;
        }
        BCACHE.buf[0].prev = &mut BCACHE.buf[NBUF - 1] as *mut Buf;

        BCACHE.head = &mut BCACHE.buf[0] as *mut Buf;
    }
}

fn bget(dev: u32, block_no: u32) -> *mut Buf {
    unsafe {
        // checking the most recently used buffers first
        let mut b = BCACHE.head;
        for _ in 0..NBUF {
            if (*b).dev == dev && (*b).block_no == block_no {
                (*b).ref_cnt += 1;
                return b;
            }
            b = (*b).next;
        }

        // if not cached, picks the least recently used buffer
        b = (*BCACHE.head).prev;
        for _ in 0..NBUF {
            if (*b).ref_cnt == 0 {
                (*b).valid = 0;
                (*b).ref_cnt = 1;
                (*b).dev = dev;
                (*b).block_no = block_no;
                return b;
            }
            b = (*b).prev;
        }
    }
    panicc!("bget: no buffer available");
}

pub fn bread(dev: u32, block_no: u32) -> *mut Buf {
    let b = bget(dev, block_no);
    unsafe {
        if (*b).valid == 0 {
            virtio_disk_rw(b, 0);
            (*b).valid = 1;
        }
    }
    b
}

pub fn bwrite(b: *mut Buf) {
    virtio_disk_rw(b, 1);
}

pub fn brelse(b: *mut Buf) {
    unsafe {
        (*b).ref_cnt -= 1;

        // move the buffer to the front of the linked list
        if (*b).ref_cnt == 0 {
            (*(*b).prev).next = (*b).next;
            (*(*b).next).prev = (*b).prev;
            (*b).prev = (*BCACHE.head).prev;
            (*b).next = (*BCACHE.head).next;
            (*(*b).prev).next = b;
            (*BCACHE.head).prev = b;
            BCACHE.head = b;
        }
    }
}
