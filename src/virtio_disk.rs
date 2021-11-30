use crate::block_cache::Buf;
use crate::fs::BLOCK_SIZE;
use crate::riscv::{PAGE_SHIFT, PAGE_SIZE};
use core::fmt::Write;
use core::mem::size_of;
use core::ptr::null_mut;

// https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.pdf
// pdf 4.2.2
pub const VIRTIO_MMIO_MAGIC_VALUE: *const u32 = 0x1000_1000 as *const u32;
pub const VIRTIO_MMIO_VERSION: *const u32 = 0x1000_1004 as *const u32;
pub const VIRTIO_MMIO_DEVICE_ID: *const u32 = 0x1000_1008 as *const u32;
pub const VIRTIO_MMIO_VENDOR_ID: *const u32 = 0x1000_100c as *const u32;
pub const VIRTIO_MMIO_DEVICE_FEATURES: *const u32 = 0x1000_1010 as *const u32;
pub const VIRTIO_MMIO_DRIVER_FEATURES: *mut u32 = 0x1000_1020 as *mut u32;
pub const VIRTIO_MMIO_GUEST_PAGE_SIZE: *mut u32 = 0x1000_1028 as *mut u32;
pub const VIRTIO_MMIO_QUEUE_SEL: *mut u32 = 0x1000_1030 as *mut u32;
pub const VIRTIO_MMIO_QUEUE_NUM_MAX: *const u32 = 0x1000_1034 as *const u32;
pub const VIRTIO_MMIO_QUEUE_NUM: *mut u32 = 0x1000_1038 as *mut u32;
pub const VIRTIO_MMIO_QUEUE_ALIGN: *mut u32 = 0x1000_103c as *mut u32;
pub const VIRTIO_MMIO_QUEUE_PFN: *mut u32 = 0x1000_1040 as *mut u32;
pub const VIRTIO_MMIO_QUEUE_READY: u64 = 0x1000_1044;
pub const VIRTIO_MMIO_QUEUE_NOTIFY: *mut u32 = 0x1000_1050 as *mut u32;
pub const VIRTIO_MMIO_INTERRUPT_STATUS: *mut u32 = 0x1000_1060 as *mut u32;
pub const VIRTIO_MMIO_INTERRUPT_ACK: *mut u32 = 0x1000_1064 as *mut u32;
pub const VIRTIO_MMIO_STATUS: *mut u32 = 0x1000_1070 as *mut u32;

pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
pub const VIRTIO_CONFIG_S_DRIVER: u32 = 2;
pub const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 4;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 8;

// pdf 5.2.3
// block device feature bits
pub const VIRTIO_BLK_F_RO: u32 = 5;
pub const VIRTIO_BLK_F_SCSI: u32 = 7;
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11;
pub const VIRTIO_BLK_F_MQ: u32 = 12;
pub const VIRTIO_F_ANY_LAYOUT: u32 = 27;
pub const VIRTIO_RING_F_INDIRECT_DESC: u32 = 28;
pub const VIRTIO_RING_F_EVENT_IDX: u32 = 29;

pub const QUEUE_SIZE: u64 = 8;

// pdf 2.6
pub const VIRTQ_DESC_F_NEXT: u16 = 1; // marks a buffer as continuing via the next field
pub const VIRTQ_DESC_F_WRITE: u16 = 2; // marks a buffer as device write-only
struct VirtqDesc {
    addr: u64, // guest physical
    len: u32,
    flags: u16, // the flags as indicated above
    next: u16,  // next field if flags & NEXT
}

#[repr(C)]
struct VirtqAvail {
    flags: u16,

    // indicates where the driver would put the next descriptor entry in the ring
    // (modulo the queue size)
    idx: u16,

    // each ring entry refers to the head of a descriptor chain
    ring: [u16; QUEUE_SIZE as usize],
    used_event: u16,
}

#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; QUEUE_SIZE as usize],
}

#[repr(C)]
struct VirtqUsedElem {
    id: u32,  // index of start of used descriptor chain
    len: u32, // total length of the descriptor chain which was used (written to)
}

#[repr(C, align(4096))] // shuold be PAGE_SIZE here
struct Disk {
    /*-- virt queue --*/
    // contiguous memory for queue
    pages: [u8; 2 * PAGE_SIZE as usize],

    // The actual descriptors (16 bytes each)
    desc: *mut VirtqDesc,

    // A ring of available descriptor heads with free-running index.
    avail: *mut VirtqAvail,

    // A ring of used descriptor heads with free-running index.
    used: *mut VirtqUsed,

    /*-- other things for convenience --*/
    // record free descriptors
    is_free: [u8; QUEUE_SIZE as usize],

    used_idx: u16,

    req: [VirtioBlkReq; QUEUE_SIZE as usize],

    // written by device describing the status after a request
    status: [u8; QUEUE_SIZE as usize],
}

static mut DISK: Disk = Disk {
    pages: [0; 2 * PAGE_SIZE as usize],
    desc: null_mut(),
    avail: null_mut(),
    used: null_mut(),
    is_free: [1; QUEUE_SIZE as usize],
    used_idx: 0,
    req: [VirtioBlkReq::new(); QUEUE_SIZE as usize],
    status: [0; QUEUE_SIZE as usize],
};

// pdf 5.2.6
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_T_FLUSH: u32 = 4;

// status
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

#[repr(C)] // addr used by rw
#[derive(Copy, Clone)]
struct VirtioBlkReq {
    req_type: u32,
    reserved: u32,
    // The sector number indicates the offset (multiplied by 512)
    // where the read or write is to occur.
    sector: u64,
    // u8 data[];
    // u8 status;
}

impl VirtioBlkReq {
    const fn new() -> VirtioBlkReq {
        VirtioBlkReq {
            req_type: 0,
            reserved: 0,
            sector: 0,
        }
    }
}

fn alloc_desc() -> usize {
    for i in 0..QUEUE_SIZE as usize {
        unsafe {
            if DISK.is_free[i] == 1 {
                DISK.is_free[i] = 0;
                return i;
            }
        }
    }

    QUEUE_SIZE as usize
}

fn alloc_3desc(idx: &mut [usize; 3]) -> i32 {
    for i in 0..3 {
        let index = alloc_desc();
        if index >= QUEUE_SIZE as usize {
            for j in 0..i {
                free_desc(idx[j]);
            }
            return -1;
        }
        idx[i] = index;
    }

    0
}

// do more in xv6
fn free_desc(i: usize) {
    unsafe {
        DISK.is_free[i] = 1;
    }
}

fn free_chain(mut i: usize) {
    loop {
        let flags;
        let next;
        unsafe {
            flags = (*DISK.desc.add(i)).flags;
            next = (*DISK.desc.add(i)).next;
        }
        free_desc(i);
        match flags & VIRTQ_DESC_F_NEXT {
            0 => break,
            _ => i = next as usize,
        }
    }
}

pub fn virtio_disk_init() {
    unsafe {
        if *VIRTIO_MMIO_MAGIC_VALUE != 0x74726976
            || *VIRTIO_MMIO_VERSION != 1
            || *VIRTIO_MMIO_DEVICE_ID != 2
            || *VIRTIO_MMIO_VENDOR_ID != 0x554d4551
        {
            panicc!("no virtio disk");
        }

        let mut status: u32 = 0;
        // set ACKNOWLEDGE bit
        status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
        *VIRTIO_MMIO_STATUS = status;

        // set DRIVER status bit
        status |= VIRTIO_CONFIG_S_DRIVER;
        *VIRTIO_MMIO_STATUS = status;

        // write feature bits
        let mut features: u32 = *VIRTIO_MMIO_DEVICE_FEATURES;
        features &= !(1 << VIRTIO_BLK_F_RO);
        features &= !(1 << VIRTIO_BLK_F_SCSI);
        features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
        features &= !(1 << VIRTIO_BLK_F_MQ);
        features &= !(1 << VIRTIO_F_ANY_LAYOUT);
        features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
        features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
        *VIRTIO_MMIO_DRIVER_FEATURES = features;

        // set the FEATURES_OK status bit
        status |= VIRTIO_CONFIG_S_FEATURES_OK;
        *VIRTIO_MMIO_STATUS = status;

        // ensure FEATURES_OK bit is still set
        if *(VIRTIO_MMIO_STATUS) & VIRTIO_CONFIG_S_FEATURES_OK == 0 {
            panicc!("virtio_disk_init: features aren't supported");
        }

        // set the DRIVER_OK status bit
        status |= VIRTIO_CONFIG_S_DRIVER_OK;
        *VIRTIO_MMIO_STATUS = status;

        // used by device to calculate the Guest address of the first queue page
        *VIRTIO_MMIO_GUEST_PAGE_SIZE = PAGE_SIZE as u32;

        // select the virtual queue that the following operations apply to
        *VIRTIO_MMIO_QUEUE_SEL = 0;

        // the max size of the queue
        let max_num = *VIRTIO_MMIO_QUEUE_NUM_MAX;
        if max_num == 0 {
            panicc!("virtio_disk_init: queue not available");
        }
        if max_num < QUEUE_SIZE as u32 {
            panicc!("virtio_disk_init: max size to small");
        }
        *VIRTIO_MMIO_QUEUE_NUM = QUEUE_SIZE as u32;

        // set guest physical page number of the virtual queue
        *VIRTIO_MMIO_QUEUE_PFN = (&DISK.pages as *const u8 as u32) >> PAGE_SHIFT;

        DISK.desc = &mut DISK.pages as *mut u8 as u64 as *mut VirtqDesc;
        DISK.avail = (&mut DISK.pages as *mut u8).add(QUEUE_SIZE as usize * size_of::<VirtqDesc>())
            as *mut VirtqAvail;
        DISK.used = (&mut DISK.pages as *mut u8).add(PAGE_SIZE as usize) as *mut VirtqUsed;
    }
}

pub fn virtio_disk_rw(buf: *mut Buf, write: u32) {
    let mut idx: [usize; 3] = [0; 3];

    // xv6 wait until find 3 desc here
    if alloc_3desc(&mut idx) == -1 {
        panicc!("virtio_disk_rw: no free desc");
    }

    unsafe {
        let req = &mut DISK.req[idx[0]];

        req.req_type = match write {
            0 => VIRTIO_BLK_T_IN,
            _ => VIRTIO_BLK_T_OUT,
        };

        req.reserved = 0;
        req.sector = (*buf).block_no as u64 * (BLOCK_SIZE / 512);

        (*DISK.desc.add(idx[0])).addr = req as *mut VirtioBlkReq as u64;
        (*DISK.desc.add(idx[0])).len = size_of::<VirtioBlkReq>() as u32;
        (*DISK.desc.add(idx[0])).flags = VIRTQ_DESC_F_NEXT;
        (*DISK.desc.add(idx[0])).next = idx[1] as u16;

        (*DISK.desc.add(idx[1])).addr = &mut (*buf).data as *mut u8 as u64;
        (*DISK.desc.add(idx[1])).len = BLOCK_SIZE as u32;
        (*DISK.desc.add(idx[1])).flags = match write {
            0 => VIRTQ_DESC_F_WRITE,
            _ => 0,
        };
        (*DISK.desc.add(idx[1])).flags |= VIRTQ_DESC_F_NEXT;
        (*DISK.desc.add(idx[1])).next = idx[2] as u16;

        DISK.status[idx[0]] = 0xf; // written by the device
        (*DISK.desc.add(idx[2])).addr = &mut DISK.status[idx[0]] as *mut u8 as u64;
        (*DISK.desc.add(idx[2])).len = 1;
        (*DISK.desc.add(idx[2])).flags = VIRTQ_DESC_F_WRITE;
        (*DISK.desc.add(idx[2])).next = 0;

        // write the desc index into the available ring
        (*DISK.avail).ring[(*DISK.avail).idx as usize % QUEUE_SIZE as usize] = idx[0] as u16;
        (*DISK.avail).idx += 1; // even it overflows the res seems to stay the same (max_u16+1 % 8 = 0)

        // notify the device that there are new buffers to process in a queue
        // the value written is the queue index (when..)
        *VIRTIO_MMIO_QUEUE_NOTIFY = 0;

        // wait for intr to say finished, just a loop yet
        let mut timer: u64 = 0;
        while timer < 1_000_000 {
            timer += 1;
        }

        free_chain(idx[0]);
    }
}

pub fn virtio_disk_intr() {
    unsafe {
        // notify the device that events causing the interrupt have been handled
        *VIRTIO_MMIO_INTERRUPT_ACK = *VIRTIO_MMIO_INTERRUPT_STATUS & 0x3;

        // When the device has finished a buffer,
        // it writes the descriptor index into the used ring
        while DISK.used_idx != (*DISK.used).idx {
            let id = (*DISK.used).ring[(*DISK.used).idx as usize % QUEUE_SIZE as usize].id as usize;

            if DISK.status[id] != VIRTIO_BLK_S_OK {
                match DISK.status[id] {
                    VIRTIO_BLK_S_IOERR => {
                        panicc!("virtio_disk_intr: device or driver error");
                    }
                    VIRTIO_BLK_S_UNSUPP => {
                        panicc!("virtio_disk_intr: request unsupported by device");
                    }
                    _ => {
                        panicc!("virtio_disk_intr: unknown status");
                    }
                }
            }

            // wake up..

            DISK.used_idx += 1;
        }
    }
}
