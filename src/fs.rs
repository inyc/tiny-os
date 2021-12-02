use crate::block_cache::{bread, brelse, bwrite, Buf};
use crate::param::{NINODE, ROOT_DEV};
use crate::string::{mem_copy, str_cmp};
use core::cmp::min;
use core::fmt::Write;
use core::mem::size_of;
use core::ptr::null_mut;

// some troublesome things need to be handled...
pub const BLOCK_SIZE: u32 = 1024;
const ROOT_INO: u32 = 1;
const MAGIC: u16 = 0x4d5a;

#[repr(C)]
struct SuperBlock {
    ninode: u32,
    pad0: u16, // unused
    imap_blk_num: u16,
    zmap_blk_num: u16,
    first_data_zone: u16,
    log2_bz: u16, // log2(block/zone)
    pad1: u16,
    max_fsize: u32,
    nzone: u32,
    magic: u16,
    pad2: u16,
    block_size: u16,
    fsv: u8, // FS sub-version
}

static mut SB: SuperBlock = SuperBlock {
    ninode: 0,
    pad0: 0,
    imap_blk_num: 0,
    zmap_blk_num: 0,
    first_data_zone: 0,
    log2_bz: 0,
    pad1: 0,
    max_fsize: 0,
    nzone: 0,
    magic: 0,
    pad2: 0,
    block_size: 0,
    fsv: 0,
};

pub fn fs_init(dev: u32) {
    // the superblock is loaded from the second 1 KB of the disk device
    let b = bread(dev, 1);
    unsafe {
        mem_copy(
            &mut SB as *mut SuperBlock as u64 as *mut u64,
            &mut (*b).data as *mut u8 as *mut u64,
            size_of::<SuperBlock>() as u64,
        );

        if SB.magic != MAGIC {
            panicc!("fs_init: magic number invalid");
        }

        // nightmare...
        if SB.log2_bz != 0 {
            println!("{}", SB.log2_bz);
            panicc!("fs_init: log2_bz");
        }

        if SB.block_size != BLOCK_SIZE as u16 {
            println!("{}", SB.block_size);
            panicc!("fs_init: block_size");
        }
    }

    brelse(b);
}

const BPERB: u32 = 8 * BLOCK_SIZE; // bits per block

// alloc a block
fn balloc(dev: u32) -> u32 {
    unsafe {
        let zmap_start = 2 + SB.imap_blk_num as u32;
        for i in 0..SB.zmap_blk_num as usize {
            let b = bread(dev, zmap_start + i as u32);

            for j in 0..BPERB as usize {
                let bits = 1 << (j % 8);
                if (*b).data[j / 8] & bits == 0 {
                    (*b).data[j / 8] |= bits;
                    brelse(b);
                    return i as u32 * BPERB + j as u32;
                }
            }

            brelse(b);
        }
    }

    panicc!("balloc: no free block");
}

// free a block
fn bfree(dev: u32, block_no: u32) {
    unsafe {
        let map_bno = 2 + SB.imap_blk_num as u32 + block_no / BPERB;
        let b = bread(dev, map_bno);
        let byte_no = (block_no % BPERB / 8) as usize;
        let bit_no = block_no % BPERB % 8;
        if (*b).data[byte_no] & 1 << bit_no == 0 {
            panicc!("bfree: block is free");
        }
        (*b).data[byte_no] &= !(1 << bit_no);
        brelse(b);
    }
}

// from https://github.com/Stichting-MINIX-Research-Foundation/minix
const TYPE: u16 = 0o170000; // this field gives inode type
const SYMBOLIC_LINK: u16 = 0o140000;
const REGULAR: u16 = 0o100000; // regular file, not dir or special
const DIRECTORY: u16 = 0o040000;
const NAMED_PIPE: u16 = 0o010000; // named pipe (FIFO)
const NOT_ALLOC: u16 = 0o000000; // this node is free

const fn is_reg(m: u16) -> bool {
    m & TYPE == REGULAR
}

const fn is_dir(m: u16) -> bool {
    m & TYPE == DIRECTORY
}

const fn not_alloc(m: u16) -> bool {
    m & TYPE == NOT_ALLOC
}

const NDIRECT: usize = 7; // direct block num in an inode
const NINDIRECT: usize = BLOCK_SIZE as usize / size_of::<u32>(); // indirect block num
const NDINDIRECT: usize = (BLOCK_SIZE as usize / size_of::<u32>()) * NINDIRECT; // double indirect block num
const IPERB: u32 = (BLOCK_SIZE as usize / size_of::<InodeDisk>()) as u32;

// block number for inode
const fn iblock(ino: u32, istart: u32) -> u32 {
    ino / IPERB + istart
}

#[repr(C)]
struct InodeDisk {
    mode: u16, // file type and rwx bits
    nlink: u16,
    uid: u16, // identifies user who owns file
    gid: u16, // owner's group
    fsize: u32,
    atime: u32, // access time
    mtime: u32, // modification time
    ctime: u32, // status change time

    // 0-6 first seven data zones
    // 7 indirect zone
    // 8 double indirect zone
    // 9 unused (could be used for triple indirect zone)
    zone: [u32; NDIRECT + 3],
}

// reprC?
#[derive(Copy, Clone)]
pub struct InodeMem {
    dev: u32,
    ino: u32,
    ref_cnt: u32,
    valid: u32,

    mode: u16,
    nlink: u16,
    uid: u16,
    gid: u16,
    fsize: u32,
    atime: u32,
    mtime: u32,
    ctime: u32,
    zone: [u32; NDIRECT + 3],
}

impl InodeMem {
    pub const fn new() -> Self {
        InodeMem {
            dev: 0,
            ino: 0,
            ref_cnt: 0,
            valid: 0,

            mode: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            fsize: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            zone: [0; NDIRECT + 3],
        }
    }
}

struct Icache {
    inode: [InodeMem; NINODE],
}

static mut ICACHE: Icache = Icache {
    inode: [InodeMem::new(); NINODE],
};

// write inode to disk
pub fn iupdate(inode: *const InodeMem) {
    unsafe {
        let b = bread(
            (*inode).dev,
            iblock((*inode).ino, 2 + (SB.imap_blk_num + SB.zmap_blk_num) as u32),
        );
        let mut dinode =
            (&mut (*b).data as *mut u8 as *mut InodeDisk).add(((*inode).ino % IPERB) as usize);
        (*dinode).mode = (*inode).mode;
        (*dinode).nlink = (*inode).nlink;
        (*dinode).uid = (*inode).uid;
        (*dinode).gid = (*inode).gid;
        (*dinode).fsize = (*inode).fsize;
        (*dinode).atime = (*inode).atime;
        (*dinode).mtime = (*inode).mtime;
        (*dinode).ctime = (*inode).ctime;
        mem_copy(
            &mut (*dinode).zone as *mut u32 as *mut u64,
            &(*inode).zone as *const u32 as *const u64,
            (size_of::<u32>() * (NDIRECT + 3)) as u64,
        );
        bwrite(b);
        brelse(b);
    }
}

// read from disk if necessary
pub fn iget(dev: u32, ino: u32) -> *mut InodeMem {
    let mut empty_idx = NINODE;
    unsafe {
        for i in 0..NINODE {
            if ICACHE.inode[i].ref_cnt != 0
                && ICACHE.inode[i].dev == dev
                && ICACHE.inode[i].ino == ino
            {
                ICACHE.inode[i].ref_cnt += 1;
                return &mut ICACHE.inode[i] as *mut InodeMem;
            }

            if empty_idx == NINODE && ICACHE.inode[i].ref_cnt == 0 {
                empty_idx = i;
            }
        }

        if empty_idx == NINODE {
            panicc!("iget: no free inodes");
        }

        ICACHE.inode[empty_idx].dev = dev;
        ICACHE.inode[empty_idx].ino = ino;
        ICACHE.inode[empty_idx].ref_cnt = 1;

        let b = bread(
            dev,
            iblock(ino, 2 + (SB.imap_blk_num + SB.zmap_blk_num) as u32),
        );

        let dinode = (&(*b).data as *const u8 as *const InodeDisk).add((ino % IPERB) as usize);
        ICACHE.inode[empty_idx].mode = (*dinode).mode;
        ICACHE.inode[empty_idx].nlink = (*dinode).nlink;
        ICACHE.inode[empty_idx].uid = (*dinode).uid;
        ICACHE.inode[empty_idx].gid = (*dinode).gid;
        ICACHE.inode[empty_idx].fsize = (*dinode).fsize;
        ICACHE.inode[empty_idx].atime = (*dinode).atime;
        ICACHE.inode[empty_idx].mtime = (*dinode).mtime;
        ICACHE.inode[empty_idx].ctime = (*dinode).ctime;
        mem_copy(
            &mut ICACHE.inode[empty_idx].zone as *mut u32 as *mut u64,
            &(*dinode).zone as *const u32 as *const u64,
            (size_of::<u32>() * (NDIRECT + 3)) as u64,
        );
        brelse(b);

        ICACHE.inode[empty_idx].valid = 1;
        if not_alloc((*dinode).mode) {
            panicc!("iget: inode not alloc");
        }

        &mut ICACHE.inode[empty_idx] as *mut InodeMem
    }
}

// get the addr of the nth block in inode
// return 0 if not exist and alloc==0
fn bmap(inode: *mut InodeMem, mut bn: usize, alloc: u32) -> u32 {
    unsafe {
        if bn < NDIRECT {
            let addr = (*inode).zone[bn as usize];
            if addr == 0 && alloc != 0 {
                (*inode).zone[bn as usize] = balloc((*inode).dev);
            }
            return addr;
        }

        bn -= NDIRECT;
        let mut b: *mut Buf;
        let mut addr: u32;
        let mut ap: *mut u32;
        if bn < NINDIRECT {
            addr = (*inode).zone[NDIRECT];
            if addr == 0 {
                if alloc == 0 {
                    return 0;
                }

                addr = balloc((*inode).dev);
                (*inode).zone[NDIRECT] = addr;
            }

            b = bread((*inode).dev, addr);
            ap = &mut (*b).data as *mut u8 as *mut u32;
            addr = *ap.add(bn);
            brelse(b);

            if addr == 0 && alloc != 0 {
                addr = balloc((*inode).dev);
                *ap.add(bn) = addr;
            }

            return addr;
        }

        bn -= NINDIRECT;
        if bn < NDINDIRECT {
            addr = (*inode).zone[NDIRECT + 1];
            if addr == 0 {
                if alloc == 0 {
                    return 0;
                }

                addr = balloc((*inode).dev);
                (*inode).zone[NDIRECT + 1] = addr;
            }

            b = bread((*inode).dev, addr);
            ap = &mut (*b).data as *mut u8 as *mut u32;
            addr = *ap.add(bn / NINDIRECT);
            if addr == 0 {
                if alloc == 0 {
                    return 0;
                }

                addr = balloc((*inode).dev);
                *ap.add(bn / NINDIRECT) = addr;
            }

            b = bread((*inode).dev, addr);
            ap = &mut (*b).data as *mut u8 as *mut u32;
            addr = *ap.add(bn % NINDIRECT);
            brelse(b);

            if addr == 0 && alloc != 0 {
                addr = balloc((*inode).dev);
                *ap.add(bn % NINDIRECT) = addr;
            }

            return addr;
        }
    }

    panicc!("bmap: bn out of range");
}

// read file content from inode
pub fn readi(inode: *mut InodeMem, is_uaddr: u32, mut dst: u64, mut off: u32, mut n: u32) -> u32 {
    if is_uaddr != 0 {
        panicc!("readi: not support user addr");
    }
    unsafe {
        if off > (*inode).fsize || n > 0xffffffff - off {
            return 0;
        }

        if off + n > (*inode).fsize {
            n = (*inode).fsize - off;
        }
    }

    let mut cnt: u32 = 0;
    let mut b: *mut Buf;
    let mut data_size;
    let mut block_no;
    while cnt < n {
        block_no = bmap(inode, (off / BLOCK_SIZE) as usize, 0);
        if block_no == 0 {
            panicc!("readi: block not exist");
        }
        unsafe {
            b = bread((*inode).dev, block_no);

            data_size = min(n - cnt, BLOCK_SIZE - off % BLOCK_SIZE);
            // not for uaddr yet
            mem_copy(
                dst as *mut u64,
                (&mut (*b).data as *mut u8).add((off % BLOCK_SIZE) as usize) as *mut u64,
                data_size as u64,
            );
        }
        brelse(b);

        cnt += data_size;
        off += data_size;
        dst += data_size as u64;
    }

    cnt
}

// wirte file content in inode
pub fn writei(inode: *mut InodeMem, is_uaddr: u32, mut src: u64, mut off: u32, mut n: u32) -> u32 {
    if is_uaddr != 0 {
        panicc!("writei: not support user addr");
    }

    let mut cnt: u32 = 0;
    let mut b: *mut Buf;
    let mut data_size;
    let mut block_no;
    while cnt < n {
        block_no = bmap(inode, (off / BLOCK_SIZE) as usize, 1);
        unsafe {
            b = bread((*inode).dev, block_no);
            data_size = min(BLOCK_SIZE - off % BLOCK_SIZE, n - cnt);
            mem_copy(
                (&mut (*b).data as *mut u8).add((off % BLOCK_SIZE) as usize) as *mut u64,
                src as *mut u64,
                data_size as u64,
            );
        }
        bwrite(b);
        brelse(b);

        cnt += data_size;
        off += data_size;
        src += data_size as u64;
    }

    unsafe {
        if cnt > (*inode).fsize {
            (*inode).fsize = cnt;
        }
    }

    iupdate(inode);

    cnt
}

const FNAME_SIZE: usize = 60; // include '\0'

#[repr(C)]
struct DirEntry {
    ino: u32,
    name: [u8; FNAME_SIZE],
}

impl DirEntry {
    pub const fn new() -> Self {
        DirEntry {
            ino: 0,
            name: [0; FNAME_SIZE],
        }
    }
}

// look up for name in directory, set off (offp points to)
fn dir_lookup(inode: *mut InodeMem, name: *mut u8, offp: *mut u32) -> *mut InodeMem {
    unsafe {
        if !is_dir((*inode).mode) {
            panicc!("dir_lookup: not dir");
        }

        let mut de = DirEntry::new();
        let mut off = 0;
        while off < (*inode).fsize {
            let ret = readi(
                inode,
                0,
                &mut de as *mut DirEntry as u64,
                off,
                size_of::<DirEntry>() as u32,
            );
            if ret == 0 {
                panicc!("dir_lookup: read inode");
            }
            off += size_of::<DirEntry>() as u32;

            if de.ino == 0 {
                continue;
            }

            if str_cmp(name, &de.name as *const u8, FNAME_SIZE as u32) == 0 {
                if !offp.is_null() {
                    (*offp) = off;
                }
                return iget((*inode).dev, de.ino);
            }
        }
    }

    null_mut()
}

// move path ptr, and copy filename to name
fn eat_path(mut path: *mut u8, name: *mut u8) -> *mut u8 {
    unsafe {
        while (*path) == '/' as u8 {
            path = path.add(1);
        }

        if (*path) == 0 {
            return null_mut();
        }

        let s = path;
        let mut len: usize = 0;
        while (*path) != '/' as u8 && (*path) != 0 {
            path = path.add(1);
            len += 1;
        }

        if len >= FNAME_SIZE {
            panicc!("eat_path: file name too long");
        }

        mem_copy(name as *mut u64, s as *mut u64, len as u64);
        *name.add(len) = 0;
    }

    path
}

fn path_lookup(mut path: *mut u8) -> *mut InodeMem {
    // not support relative path yet
    let mut name: [u8; FNAME_SIZE] = [0; FNAME_SIZE];
    let mut inode = iget(ROOT_DEV, ROOT_INO);
    let mut child;
    unsafe {
        if (*path) != '/' as u8 {
            panicc!("path_lookup: not support relative path yet");
        }

        path = eat_path(path, &mut name as *mut u8);
        while !path.is_null() {
            if !is_dir((*inode).mode) {
                return null_mut();
            }

            child = dir_lookup(inode, &mut name as *mut u8, null_mut());
            if child.is_null() {
                return null_mut();
            }

            inode = child;
            path = eat_path(path, &mut name as *mut u8);
        }
    }

    inode
}
