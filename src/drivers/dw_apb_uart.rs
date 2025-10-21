//!
//! DesignWare UART Driver
//!
use crate::serial;

use core::fmt::Error;
use core::ptr;

pub struct DwApbUart {
    base_address: usize,
}

const DW_APB_UART_SIZE: usize = 0x100;
const DW_APB_UART_RHR: usize = 0x00;
const DW_APB_UART_THR: usize = 0x00;
const DW_APB_UART_IER: usize = 0x04;
const DW_APB_UART_IER_ERBFI: u32 = 1;
const DW_APB_UART_FCR: usize = 0x08;
const DW_APB_UART_FCR_FIFOE: u32 = 1;
const DW_APB_UART_MCR: usize = 0x10;
const DW_APB_UART_MCR_OUT2: u32 = 1 << 3;
const DW_APB_UART_LSR: usize = 0x14;
const DW_APB_UART_LSR_TEMT: u32 = 1 << 6;
const DW_APB_UART_LSR_DR: u32 = 1;

impl DwApbUart {
    pub const fn invalid() -> Self {
        Self { base_address: 0 }
    }

    pub fn new(base_address: usize, range: usize) -> Result<Self, ()> {
        if range < DW_APB_UART_SIZE {
            return Err(());
        }
        Ok(Self { base_address })
    }

    fn is_tx_fifo_full(&self) -> bool {
        (unsafe { ptr::read_volatile((self.base_address + DW_APB_UART_LSR) as *const u32) }
            & DW_APB_UART_LSR_TEMT)
            == 0
    }

    fn is_rx_fifo_empty(&self) -> bool {
        (unsafe { ptr::read_volatile((self.base_address + DW_APB_UART_LSR) as *const u32) }
            & DW_APB_UART_LSR_DR)
            == 0
    }

    pub fn enable_interrupt(&self) {
        unsafe {
            ptr::write_volatile(
                (self.base_address + DW_APB_UART_FCR) as *mut u32,
                DW_APB_UART_FCR_FIFOE,
            );
            ptr::write_volatile(
                (self.base_address + DW_APB_UART_IER) as *mut u32,
                DW_APB_UART_IER_ERBFI,
            );
            ptr::write_volatile(
                (self.base_address + DW_APB_UART_MCR) as *mut u32,
                DW_APB_UART_MCR_OUT2,
            );
        }
    }
}

/// Serial構造体で使うために必要な実装
impl serial::SerialDevice for DwApbUart {
    fn putc(&self, c: u8) -> Result<(), Error> {
        while self.is_tx_fifo_full() {
            core::hint::spin_loop();
        }
        unsafe { ptr::write_volatile((self.base_address + DW_APB_UART_THR) as *mut u32, c as u32) };
        Ok(())
    }

    fn getc(&self) -> Result<Option<u8>, Error> {
        if self.is_rx_fifo_empty() {
            return Ok(None);
        }
        Ok(Some(
            (unsafe { ptr::read_volatile((self.base_address + DW_APB_UART_RHR) as *const u32) }
                & 0xFF) as u8,
        ))
    }
}
