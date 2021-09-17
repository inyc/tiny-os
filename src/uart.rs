use core::convert::TryInto;
use core::fmt::{Error, Write};

pub struct Uart {
    base_addr: usize,
}

impl Uart {
    pub fn new(base_addr: usize) -> Uart {
        Uart { base_addr }
    }

    pub fn init(&self) {
        let ptr = self.base_addr as *mut u8;

        unsafe {
            // // reg LCR, write DLAB
            // ptr.add(3).write_volatile(1 << 7);

            // let divisor: u16 = 592;
            // let divisor_least: u8 = (divisor & 0xff).try_into().unwrap();
            // let divisor_most: u8 = (divisor >> 8).try_into().unwrap();

            // // reg DLL,DLM, divisor setting
            // ptr.add(0).write_volatile(divisor_least);
            // ptr.add(1).write_volatile(divisor_most);

            // // reg LCR, set the word length to 8 bits
            // ptr.add(3).write_volatile(1 | (1 << 1));

            // // reg FCR, enable FIFO
            // ptr.add(2).write_volatile(1);

            // // reg IER, enable received data available interrupt
            // ptr.add(1).write_volatile(1);

            ptr.add(3).write_volatile((1 << 0) | (1 << 1));

            ptr.add(2).write_volatile(1 << 0);

            ptr.add(1).write_volatile(1 << 0);

            let divisor: u16 = 592;
            let divisor_least: u8 = (divisor & 0xff).try_into().unwrap();
            let divisor_most: u8 = (divisor >> 8).try_into().unwrap();

            let lcr = ptr.add(3).read_volatile();
            ptr.add(3).write_volatile(lcr | 1 << 7);

            ptr.add(0).write_volatile(divisor_least);
            ptr.add(1).write_volatile(divisor_most);

            ptr.add(3).write_volatile(lcr);
        }
    }

    pub fn get(&self) -> Option<u8> {
        let ptr = self.base_addr as *mut u8;

        unsafe {
            // reg LSR, DR(data ready)
            if ptr.add(5).read_volatile() & 1 == 0 {
                None
            } else {
                // reg RBR(receiver buffer register)
                Some(ptr.add(0).read_volatile())
            }
        }
    }

    fn put(&self, c: u8) {
        let ptr = self.base_addr as *mut u8;

        unsafe {
            // reg THR
            ptr.add(0).write_volatile(c);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        for c in s.bytes() {
            self.put(c);
        }

        Ok(())
    }
}
