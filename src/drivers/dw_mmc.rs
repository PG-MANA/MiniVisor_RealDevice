//!
//! DesignWare MMC Driver
//!

use crate::asm::{clean_data_cache, data_barrier, flush_data_cache_all, invalidate_cache};

pub struct DwMmc {
    base_address: usize,
    fifo_depth: u32,
}

const DWMMC_MMIO_SIZE: usize = 0x4000;

// Register Offsets
const DWMMC_CONTROL: usize = 0x00;
const DWMMC_TIMEOUT: usize = 0x14;
const DWMMC_BLOCK_SIZE: usize = 0x1C;
const DWMMC_BYTE_COUNT: usize = 0x20;
const DWMMC_CMD_ARG: usize = 0x28;
const DWMMC_CMD: usize = 0x2C;
const DWMMC_RESPONSE: usize = 0x30;
const DWMMC_RAW_INTERRUPT_STATUS: usize = 0x44;
const DWMMC_STATUS: usize = 0x48;
const DWMMC_FIFO_THRESHOLD: usize = 0x4C;
const DWMMC_BUS_MODE: usize = 0x80;
const DWMMC_DATA: usize = 0x200;

// Control
const DWMMC_CONTROL_RESET_FIFO: u32 = 1 << 1;

// Command Attributes
const DWMMC_CMD_START: u32 = 1 << 31;
const DWMMC_CMD_USE_HOLD_REGISTER: u32 = 1 << 29;
const DWMMC_CMD_STOP_ABORT: u32 = 1 << 14;
const DWMMC_CMD_WAIT_PREVIOUS_DATA: u32 = 1 << 13;
const DWMMC_CMD_WRITE: u32 = 1 << 10;
const DWMMC_CMD_DATA_EXPECTED: u32 = 1 << 9;
const DWMMC_CMD_RESPONSE_CRC: u32 = 1 << 8;
const DWMMC_CMD_RESPONSE_EXPECTED: u32 = 1 << 6;

// Command Opcodes
const MMC_CMD_STOP_TRANSMISSION: u32 = 12;
const MMC_CMD_SET_BLOCK_LENGTH: u32 = 16;
const MMC_CMD_READ_SINGLE_BLOCK: u32 = 17;
const MMC_CMD_READ_MULTIPLE_BLOCK: u32 = 18;
const MMC_CMD_WRITE_SINGLE_BLOCK: u32 = 24;
const MMC_CMD_WRITE_MULTIPLE_BLOCK: u32 = 25;

// Interrupt Masks
const DWMMC_INTERRUPT_MASK_END_BIT_ERROR: u32 = 1 << 15;
const DWMMC_INTERRUPT_MASK_START_BIT_ERROR: u32 = 1 << 13;
const DWMMC_INTERRUPT_MASK_RESPONSE_TIME_OUT: u32 = 1 << 8;
const DWMMC_INTERRUPT_MASK_RECEIVE_DATA: u32 = 1 << 5;
const DWMMC_INTERRUPT_MASK_TRANSMIT_DATA: u32 = 1 << 4;
const DWMMC_INTERRUPT_MASK_DATA_TRANSFER_OVER: u32 = 1 << 3;
const DWMMC_INTERRUPT_MASK_CMD_DONE: u32 = 1 << 2;
const DWMMC_INTERRUPT_MASK_RESPONSE_ERROR: u32 = 1 << 1;

// Status
const DWMMC_STATUS_BUSY: u32 = 1 << 9;
const DWMMC_STATUS_FIFO_FULL: u32 = 1 << 3;
const DWMMC_STATUS_FIFO_EMPTY: u32 = 1 << 2;

// FIFO Threshold
const DWMMC_FIFO_THRESHOLD_MSIZE_OFFSET: usize = 28;
const DWMMC_FIFO_THRESHOLD_RX_WATERMARK_OFFSET: usize = 16;
const DWMMC_FIFO_THRESHOLD_TX_WATERMARK_OFFSET: usize = 0;

// Bus Modes
const DWMMC_BUS_MODE_IDMAC_ENABLE: u32 = 1 << 7;
const DWMMC_BUS_MODE_IDMAC_FIXED_BURST: u32 = 1 << 1;

impl DwMmc {
    pub const fn invalid() -> Self {
        Self {
            base_address: 0,
            fifo_depth: 0,
        }
    }

    pub fn new(base_address: usize, mmio_size: usize, fifo_depth: u32) -> Result<Self, ()> {
        if mmio_size < DWMMC_MMIO_SIZE {
            return Err(());
        }
        println!(
            "DWMMC: Base: {base_address:#X}, Size: {mmio_size:#X}, FIFO Depth: {fifo_depth:#X}"
        );
        let mmc = Self {
            base_address,
            fifo_depth,
        };

        mmc.change_block_size(1 << 9).map_err(|_| ())?;

        Ok(mmc)
    }

    fn read_register(base_address: usize, offset: usize) -> u32 {
        unsafe {
            invalidate_cache(base_address + offset);
            core::ptr::read_volatile((base_address + offset) as *const u32)
        }
    }

    fn write_register(base_address: usize, offset: usize, data: u32) {
        unsafe {
            core::ptr::write_volatile((base_address + offset) as *mut u32, data);
            clean_data_cache(base_address + offset, size_of::<u32>());
        }
    }

    fn send_command(&self, command: u32, arg: u32) -> Result<u32, u32> {
        /* Wait and Reset */
        while (Self::read_register(self.base_address, DWMMC_STATUS) & DWMMC_STATUS_BUSY) != 0 {
            core::hint::spin_loop();
        }
        Self::write_register(
            self.base_address,
            DWMMC_RAW_INTERRUPT_STATUS,
            u16::MAX as u32,
        );
        Self::write_register(self.base_address, DWMMC_TIMEOUT, u32::MAX);

        /* Write the command and argument*/
        Self::write_register(self.base_address, DWMMC_CMD_ARG, arg);
        Self::write_register(self.base_address, DWMMC_CMD, command);

        let mut status;
        loop {
            status = Self::read_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS);
            if (status & DWMMC_INTERRUPT_MASK_CMD_DONE) != 0 {
                break;
            }
            core::hint::spin_loop();
        }
        if (status & (DWMMC_INTERRUPT_MASK_RESPONSE_ERROR | DWMMC_INTERRUPT_MASK_RESPONSE_TIME_OUT))
            != 0
        {
            println!("Status Error: {status:#X}");
            Err(status)
        } else {
            Ok(status)
        }
    }

    fn change_block_size(&self, block_size: usize) -> Result<(), ()> {
        let status = self
            .send_command(
                DWMMC_CMD_START
                    | DWMMC_CMD_USE_HOLD_REGISTER
                    | DWMMC_CMD_WAIT_PREVIOUS_DATA
                    | DWMMC_CMD_RESPONSE_CRC
                    | DWMMC_CMD_RESPONSE_EXPECTED
                    | MMC_CMD_SET_BLOCK_LENGTH,
                block_size as u32,
            )
            .map_err(|status| {
                println!("Failed to change the block size: {status:#X}");
                ()
            })?;

        Self::write_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS, status);
        let _ = Self::read_register(self.base_address, DWMMC_RESPONSE);
        Ok(())
    }

    fn stop_transmission(&self) -> Result<(), ()> {
        let status = self
            .send_command(
                DWMMC_CMD_START
                    | DWMMC_CMD_USE_HOLD_REGISTER
                    | DWMMC_CMD_STOP_ABORT
                    | DWMMC_CMD_RESPONSE_CRC
                    | DWMMC_CMD_RESPONSE_EXPECTED
                    | MMC_CMD_STOP_TRANSMISSION,
                0,
            )
            .map_err(|status| {
                println!("Failed to stop the transmission: {status:#X}");
                ()
            })?;
        Self::write_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS, status);
        let _ = Self::read_register(self.base_address, DWMMC_RESPONSE);
        Ok(())
    }

    fn operation_sync(
        &mut self,
        buffer_address: usize,
        block_address: u64,
        length: u64,
        is_write: bool,
    ) -> Result<(), ()> {
        let mut should_stop_transmission = false;
        let block_size = 1 << 9;
        if (block_address & ((1 << 9) - 1)) != 0 || (length & ((1 << 9) - 1) != 0) {
            println!(
                "Block Address({:#X}) and Length({:#X}) must be 512Byte-Aligned.",
                block_address, length
            );
            return Err(());
        }

        self.change_block_size(block_size as _)?;
        data_barrier();

        /* TODO: Set up DMA */
        /*
        let data_address_low = (buffer_address & u32::MAX as usize) as u32;
        let data_address_high = (buffer_address >> 32) as u32;
        Self::write_register(self.base_address, DWMMC_DESCRIPTOR_BASE, data_address_low);
        Self::write_register(self.base_address,DWMMC_DESCRIPTOR_BASE_HIGH, data_address_high);

        Self::write_register(
            self.base_address,
            DWMMC_CONTROL,
            Self::read_register(self.base_address, DWMMC_CONTROL) | DWMMC_CONTROL_USE_IDMAC,
        );
        Self::write_register(
            self.base_address,
            DWMMC_BUS_MODE,
            Self::read_register(self.base_address, DWMMC_BUS_MODE)
                | DWMMC_BUS_MODE_IDMAC_FIXED_BURST | DWMMC_BUS_MODE_IDMAC_ENABLE,
        );
        */

        /* Set up FIFO */
        Self::write_register(self.base_address, DWMMC_BLOCK_SIZE, block_size as u32);
        Self::write_register(self.base_address, DWMMC_BYTE_COUNT, length as u32);
        let fifo_threshold = (2 << DWMMC_FIFO_THRESHOLD_MSIZE_OFFSET)
            | ((self.fifo_depth / 2 - 1) << DWMMC_FIFO_THRESHOLD_RX_WATERMARK_OFFSET)
            | ((self.fifo_depth / 2) << DWMMC_FIFO_THRESHOLD_TX_WATERMARK_OFFSET);
        Self::write_register(self.base_address, DWMMC_FIFO_THRESHOLD, fifo_threshold);
        Self::write_register(self.base_address, DWMMC_CONTROL, DWMMC_CONTROL_RESET_FIFO);
        while (Self::read_register(self.base_address, DWMMC_CONTROL) & DWMMC_CONTROL_RESET_FIFO)
            != 0
        {
            core::hint::spin_loop();
        }
        Self::write_register(
            self.base_address,
            DWMMC_BUS_MODE,
            Self::read_register(self.base_address, DWMMC_BUS_MODE)
                | DWMMC_BUS_MODE_IDMAC_FIXED_BURST & !DWMMC_BUS_MODE_IDMAC_ENABLE,
        );

        /* コマンドと引数の設定 */
        let mut cmd = DWMMC_CMD_START
            | DWMMC_CMD_USE_HOLD_REGISTER
            | DWMMC_CMD_WAIT_PREVIOUS_DATA
            | DWMMC_CMD_DATA_EXPECTED
            | DWMMC_CMD_RESPONSE_CRC
            | DWMMC_CMD_RESPONSE_EXPECTED;
        if is_write {
            if length == block_size {
                cmd |= MMC_CMD_WRITE_SINGLE_BLOCK;
            } else {
                cmd |= MMC_CMD_WRITE_MULTIPLE_BLOCK;
                should_stop_transmission = true;
            }
            cmd |= DWMMC_CMD_WRITE;
        } else {
            if length == block_size {
                cmd |= MMC_CMD_READ_SINGLE_BLOCK;
            } else {
                cmd |= MMC_CMD_READ_MULTIPLE_BLOCK;
                should_stop_transmission = true;
            }
        }
        let arg = (block_address / block_size) as u32;
        let mut retry = 3;
        data_barrier();
        loop {
            match self.send_command(cmd, arg) {
                Ok(_) => {
                    break;
                }
                Err(status) => {
                    println!(
                        "Failed to process the command(cmd: {cmd:#X}, arg: {arg:#X}): {status:#X}"
                    );
                    if (status & DWMMC_INTERRUPT_MASK_RESPONSE_TIME_OUT) != 0 && retry > 0 {
                        Self::write_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS, status);
                        println!("Retry...");
                        retry -= 1;
                        continue;
                    }
                    return Err(());
                }
            }
        }
        let _response = Self::read_register(self.base_address, DWMMC_RESPONSE);
        // println!("Response: {_response:#X}");

        /* データ送受信 */
        let mut status;
        let mut pointer = 0;
        flush_data_cache_all();
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(
                buffer_address as *mut u32,
                length as usize / size_of::<u32>(),
            )
        };
        while pointer < buffer.len() {
            status = Self::read_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS);
            if (status
                & (DWMMC_INTERRUPT_MASK_START_BIT_ERROR | DWMMC_INTERRUPT_MASK_END_BIT_ERROR))
                != 0
            {
                println!(
                    "Failed to read the data at {:#X}",
                    pointer * size_of::<u32>()
                );
                return Err(());
            }

            if is_write {
                if (status & DWMMC_INTERRUPT_MASK_TRANSMIT_DATA) == 0 {
                    continue;
                }
            } else {
                let mask =
                    DWMMC_INTERRUPT_MASK_RECEIVE_DATA | DWMMC_INTERRUPT_MASK_DATA_TRANSFER_OVER;
                if (status & mask) == 0 {
                    continue;
                }
                Self::write_register(self.base_address, DWMMC_RAW_INTERRUPT_STATUS, status & mask);
            }

            /* Wait FIFO */
            let mut available_size;
            loop {
                available_size = Self::read_register(self.base_address, DWMMC_STATUS);
                if (is_write && (available_size & DWMMC_STATUS_FIFO_FULL) == 0)
                    || (available_size & DWMMC_STATUS_FIFO_EMPTY) == 0
                {
                    break;
                }
                core::hint::spin_loop();
            }
            available_size = (available_size >> 17) & 0x1FFF;
            if is_write {
                available_size = self.fifo_depth - available_size;
            }

            data_barrier();
            for _ in 0..available_size {
                if is_write {
                    Self::write_register(self.base_address, DWMMC_DATA, buffer[pointer]);
                } else {
                    buffer[pointer] = Self::read_register(self.base_address, DWMMC_DATA);
                }
                pointer += 1;
                if pointer == buffer.len() {
                    break;
                }
            }
            if is_write {
                Self::write_register(
                    self.base_address,
                    DWMMC_RAW_INTERRUPT_STATUS,
                    DWMMC_INTERRUPT_MASK_TRANSMIT_DATA,
                );
            }
        }

        if should_stop_transmission {
            self.stop_transmission()?;
        }
        flush_data_cache_all();
        Ok(())
    }

    pub fn read(
        &mut self,
        buffer_address: usize,
        block_address: u64,
        length: u64,
    ) -> Result<(), ()> {
        self.operation_sync(buffer_address, block_address, length, false)
    }

    pub fn write(
        &mut self,
        buffer_address: usize,
        block_address: u64,
        length: u64,
    ) -> Result<(), ()> {
        self.operation_sync(buffer_address, block_address, length, true)
    }
}

unsafe impl core::marker::Send for DwMmc {}
