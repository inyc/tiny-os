use core::fmt::Write; // for println!

pub struct Table {
    entries: [Entry; 512],
}

impl Table {
    pub fn len() -> usize {
        512
    }
}

#[repr(i64)]
pub enum EntryBits {
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Execute = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Accessed = 1 << 6,
    Dirty = 1 << 7,

    ReadWrite = 1 << 1 | 1 << 2,
    ReadExecute = 1 << 1 | 1 << 3,
}

impl EntryBits {
    pub fn val(self) -> i64 {
        self as i64
    }
}

pub struct Entry {
    entry: i64,
}

impl Entry {
    pub fn is_valid(&self) -> bool {
        if self.entry & EntryBits::Valid.val() != 0 {
            true
        } else {
            false
        }
    }

    pub fn set_entry(&mut self, bits: i64) {
        self.entry = bits;
    }

    pub fn get_entry(&self) -> i64 {
        self.entry as i64
    }

    pub fn to_pa(&self) -> i64 {
        (self.entry as i64 >> 10) << 12
    }
}

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

static mut ALLOC_START: usize = 0;
pub const PAGE_SIZE: usize = 4096;

struct FreePages {
    next: *mut FreePages,
}

#[repr(u8)]
enum PageBits {
    Empty = 0,
    Taken = 1 << 0,
    Last = 1 << 1,
}

impl PageBits {
    fn val(self) -> u8 {
        self as u8
    }
}

struct Page {
    flags: u8,
}

impl Page {
    fn is_taken(&self) -> bool {
        self.flags & PageBits::Taken.val() != 0
    }

    fn is_free(&self) -> bool {
        !self.is_taken()
    }

    fn is_last(&self) -> bool {
        self.flags & PageBits::Last.val() != 0
    }

    fn free(&mut self) {
        self.flags = PageBits::Empty.val();
    }

    fn set_flag(&mut self, flag: PageBits) {
        self.flags |= flag.val();
    }
}

pub const fn align_val(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}

pub const fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub const fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

pub fn init() {
    unsafe {
        let pages_num = HEAP_SIZE / PAGE_SIZE;
        let ptr = HEAP_START as *mut Page;

        for i in 0..pages_num {
            (*ptr.add(i)).free();
        }

        ALLOC_START = page_round_up(HEAP_START + pages_num * core::mem::size_of::<Page>());
    }
}

pub fn alloc(pages: usize) -> *mut u8 {
    assert!(pages > 0);

    unsafe {
        let pages_num = HEAP_SIZE / PAGE_SIZE;
        assert!(pages_num >= pages);

        let ptr = HEAP_START as *mut Page;
        for i in 0..=pages_num - pages {
            let mut found = false;
            if (*ptr.add(i)).is_free() {
                found = true;

                for j in 0..pages {
                    if (*ptr.add(i + j)).is_taken() {
                        found = false;
                        break;
                    }
                }
            }

            if found {
                for j in i..i + pages {
                    (*ptr.add(j)).set_flag(PageBits::Taken);
                }

                (*ptr.add(i + pages - 1)).set_flag(PageBits::Last);

                return (ALLOC_START + PAGE_SIZE * i) as *mut u8;
            }
        }
    }

    core::ptr::null_mut()
}

pub fn zalloc(pages: usize) -> *mut u8 {
    let ptr = alloc(pages);

    unsafe {
        if !ptr.is_null() {
            let size = (PAGE_SIZE * pages) / 8; // core:mem::size_of::<u64>()
            let p = ptr as *mut u64;
            for i in 0..size {
                (*p.add(i)) = 0;
            }
        }
    }

    ptr
}

pub fn dealloc(ptr: *const u8) {
    assert!(!ptr.is_null());

    unsafe {
        let addr = HEAP_START + (ptr as usize - ALLOC_START) / PAGE_SIZE;
        assert!(addr >= HEAP_START && addr <= HEAP_START + HEAP_SIZE);

        let mut p = addr as *mut Page;

        while (*p).is_taken() && !(*p).is_last() {
            (*p).free();
            p = p.add(1);
        }

        assert!(
            (*p).is_last(),
            "possible double-free detected(not taken found before last)"
        );

        (*p).free();
    }
}

pub fn print_page_allocations() {
    unsafe {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let mut beg = HEAP_START as *const Page;
        let end = beg.add(num_pages);
        let alloc_beg = ALLOC_START;
        let alloc_end = ALLOC_START + num_pages * PAGE_SIZE;
        println!();
        println!("HEAP: 0x{:x} -> 0x{:x}", HEAP_START, HEAP_START + HEAP_SIZE);
        println!(
            "PAGE ALLOCATION TABLE\nMETA: {:p} -> {:p}\nPHYS: \
					0x{:x} -> 0x{:x}",
            beg, end, alloc_beg, alloc_end
        );
        println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let mut num = 0;
        while beg < end {
            if (*beg).is_taken() {
                let start = beg as usize;
                let memaddr = ALLOC_START + (start - HEAP_START) * PAGE_SIZE;
                print!("0x{:x} => ", memaddr);
                loop {
                    num += 1;
                    if (*beg).is_last() {
                        let end = beg as usize;
                        let memaddr = ALLOC_START + (end - HEAP_START) * PAGE_SIZE + PAGE_SIZE - 1;
                        print!("0x{:x}: {:>3} page(s)", memaddr, (end - start + 1));
                        println!(".");
                        break;
                    }
                    beg = beg.add(1);
                }
            }
            beg = beg.add(1);
        }
        println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        println!(
            "Allocated: {:>6} pages ({:>10} bytes).",
            num,
            num * PAGE_SIZE
        );
        println!(
            "Free     : {:>6} pages ({:>10} bytes).",
            num_pages - num,
            (num_pages - num) * PAGE_SIZE
        );
        println!();
    }
}

pub fn walk(page_table: &Table, va: usize) -> Option<usize> {
    let vpn = [(va >> 12) & 0x1ff, (va >> 21) & 0x1ff, (va >> 30) & 0x1ff];

    let mut pte = &page_table.entries[vpn[2]];
    for i in (1..2).rev() {
        if !pte.is_valid() {
            return None;
        }

        let page = pte.to_pa() as *const Table;
        unsafe {
            pte = &(*page).entries[vpn[i]];
        }
    }

    match pte.is_valid() {
        true => Some(pte.to_pa() as usize | (va & 0xfff)),
        false => None,
    }
}

pub fn map(page_table: &mut Table, va: usize, pa: usize, bits: i64, level: usize) {
    assert!(bits & 0xe != 0);

    // pground va? seems dosen't effect VPN,but offset

    let vpn = [(va >> 12) & 0x1ff, (va >> 21) & 0x1ff, (va >> 30) & 0x1ff];
    // let ppn=[(pa>>12)&0x1ff,(pa>>21)&0x1ff,(pa>>30)&0x3ff_ffff];
    let ppn = pa >> 12;

    let mut pte = &mut page_table.entries[vpn[2]];

    for i in (level..2).rev() {
        if !pte.is_valid() {
            let page = zalloc(1);
            pte.set_entry(page as i64 >> 2 | EntryBits::Valid.val());
        }

        let first_entry = pte.to_pa() as *mut Entry;
        unsafe {
            pte = first_entry.add(vpn[i]).as_mut().unwrap();
        }

        // ok, it's my code, it does cause bug
        // unsafe {
        //     let page = (pte.to_pa() as *mut Table).as_mut().unwrap();
        //     pte = &mut page.entries[vpn[i]];
        // }
    }

    pte.set_entry((ppn as i64) << 10 | bits | EntryBits::Valid.val());
}

pub fn unmap(page_2: &mut Table) {
    for lv_2 in 0..Table::len() {
        let pte_2 = &page_2.entries[lv_2];
        if pte_2.is_valid() {
            let page_1 = unsafe { (pte_2.to_pa() as *mut Table).as_mut().unwrap() };

            for lv_1 in 0..Table::len() {
                let pte_1 = &page_1.entries[lv_1];
                if pte_1.is_valid() {
                    dealloc(pte_1.to_pa() as *mut u8);
                }
            }

            dealloc(pte_2.to_pa() as *mut u8);
        }
    }
}
