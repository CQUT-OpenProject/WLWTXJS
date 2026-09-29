//! CC2530 Flash controller, DMA programming, and banked readback verification.
//!
//! The hardware sequence is rewritten from Breezio-neo's xzg_cli implementation.

use std::path::Path;
use std::time::Instant;

use crate::cc_debugger::{
    CcDebugger, TiProbeInfo, XREG_DMAARM, XREG_FADDRH, XREG_FADDRL, XREG_FCTL, XREG_FMAP,
};
use crate::error::{Error, Result};
use crate::intel_hex::{CC2530_MAX_ADDRESS, IntelHexImage};
use crate::usb::{RusbTransport, UsbTransport};

pub const FLASH_BANK_SIZE: u32 = 0x8000;
pub const XBANK_OFFSET: u32 = 0x8000;
pub const PROG_BLOCK_SIZE: usize = 1024;

const CC2530_CHIP_ID: u16 = 0x2530;
const XREG_DMA_DESC_LOW: u16 = 0x70D2;
const XREG_DMA_DESC_HIGH: u16 = 0x70D3;

#[derive(Clone, Copy, Debug, Default)]
pub struct FlashOptions {
    pub erase: bool,
    pub write: bool,
    pub verify: bool,
}

pub fn image_from_hex(image: &IntelHexImage, block_size: usize) -> Result<Vec<u8>> {
    image.validate_range(CC2530_MAX_ADDRESS)?;
    if block_size == 0 {
        return Err(Error::protocol("program block size must not be zero"));
    }
    let max_address = image.highest_address() as usize + 1;
    let padded_length = max_address.div_ceil(block_size) * block_size;
    if padded_length > CC2530_MAX_ADDRESS as usize + 1 {
        return Err(Error::hex(
            "padded image exceeds CC2530 64 KiB address range",
        ));
    }
    let mut data = vec![0xFF; padded_length];
    for section in image.sections() {
        let start = usize::from(section.address);
        let end = start + section.data.len();
        data[start..end].copy_from_slice(&section.data);
    }
    Ok(data)
}

pub fn flash_ti_debugger(firmware: &Path, options: FlashOptions) -> Result<()> {
    let hex_image = IntelHexImage::from_file(firmware)?;
    let image = image_from_hex(&hex_image, PROG_BLOCK_SIZE)?;
    let transport = RusbTransport::connect()?;
    let mut debugger = CcDebugger::new(transport);

    flash_connected_debugger(&mut debugger, &image, options)
}

fn flash_connected_debugger<T: UsbTransport>(
    debugger: &mut CcDebugger<T>,
    image: &[u8],
    options: FlashOptions,
) -> Result<()> {
    let info = debugger.probe(false)?;
    validate_target(&info)?;
    eprintln!(
        "Target CC{:04X}, debugger fw=0x{:04X}/0x{:04X}",
        info.chip_id, info.fw_version, info.fw_revision
    );

    let operation = (|| {
        if options.erase {
            debugger.chip_erase()?;
        }
        if options.write {
            write_flash_fast(debugger, image)?;
        }
        if options.verify {
            verify_image(debugger, image)?;
        }
        Ok(())
    })();

    let reset = debugger.reset(false);
    match (operation, reset) {
        (Err(error), Err(reset_error)) => {
            eprintln!("Failed to restore normal mode after flash error: {reset_error}");
            Err(error)
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            eprintln!("Target reset to normal mode");
            Ok(())
        }
    }
}

fn validate_target(info: &TiProbeInfo) -> Result<()> {
    if info.chip_id != CC2530_CHIP_ID {
        return Err(Error::protocol(format!(
            "unsupported target chip ID 0x{:04X}; expected CC2530 (0x{CC2530_CHIP_ID:04X})",
            info.chip_id
        )));
    }
    Ok(())
}

pub fn write_flash_fast<T: UsbTransport>(debugger: &mut CcDebugger<T>, image: &[u8]) -> Result<()> {
    debugger.init_debug_interface()?;
    let addr_dma_desc = 0x0800_u16;
    let ch_dbg_to_buf0 = 0x02;
    let ch_dbg_to_buf1 = 0x04;
    let ch_buf0_to_flash = 0x08;
    let ch_buf1_to_flash = 0x10;

    let dma_desc = [
        0x62, 0x60, 0x00, 0x00, 0x04, 0x00, 31, 0x11, 0x62, 0x60, 0x04, 0x00, 0x04, 0x00, 31, 0x11,
        0x00, 0x00, 0x62, 0x73, 0x04, 0x00, 18, 0x42, 0x04, 0x00, 0x62, 0x73, 0x04, 0x00, 18, 0x42,
    ];
    debugger.write_xdata(addr_dma_desc, &dma_desc)?;
    debugger.write_xdata(XREG_DMA_DESC_LOW, &[addr_dma_desc as u8])?;
    debugger.write_xdata(XREG_DMA_DESC_HIGH, &[(addr_dma_desc >> 8) as u8])?;
    debugger.write_xdata(XREG_FADDRL, &[0])?;
    debugger.write_xdata(XREG_FADDRH, &[0])?;

    let padded_length = image.len().div_ceil(PROG_BLOCK_SIZE) * PROG_BLOCK_SIZE;
    let mut padded = image.to_vec();
    padded.resize(padded_length, 0xFF);
    let total_blocks = padded.len() / PROG_BLOCK_SIZE;
    for block in 0..total_blocks {
        let (debug_arm, flash_arm) = if block % 2 == 1 {
            (ch_dbg_to_buf1, ch_buf1_to_flash)
        } else {
            (ch_dbg_to_buf0, ch_buf0_to_flash)
        };
        debugger.write_xdata(XREG_DMAARM, &[debug_arm])?;
        let start = block * PROG_BLOCK_SIZE;
        let chunk = &padded[start..start + PROG_BLOCK_SIZE];
        let mut raw = Vec::with_capacity(3 + chunk.len());
        raw.extend_from_slice(&[0xEE, 0x84, 0x00]);
        raw.extend_from_slice(chunk);
        debugger.send_raw_data(&raw)?;
        wait_flash_idle(debugger)?;
        debugger.write_xdata(XREG_DMAARM, &[flash_arm])?;
        debugger.write_xdata(XREG_FCTL, &[0x06])?;
        let percent = ((block + 1) * 100) / total_blocks;
        eprintln!("Write {percent}% ({}/{total_blocks} blocks)", block + 1);
    }
    wait_flash_idle(debugger)
}

fn wait_flash_idle<T: UsbTransport>(debugger: &mut CcDebugger<T>) -> Result<()> {
    let timing = debugger.timing();
    let deadline = Instant::now() + timing.flash_timeout;
    while Instant::now() < deadline {
        let fctl = debugger.read_xdata(XREG_FCTL, 1)?[0];
        if fctl & 0x80 == 0 {
            return Ok(());
        }
        std::thread::sleep(timing.flash_poll);
    }
    Err(Error::protocol("flash controller stayed busy"))
}

fn flash_read_start<T: UsbTransport>(debugger: &mut CcDebugger<T>) -> Result<()> {
    let _ = debugger.control_in(0xC6, 0, 0, 1)?;
    debugger.send_raw_data(&[
        0x40, 0x55, 0x00, 0x72, 0x56, 0xE5, 0xD0, 0x74, 0x56, 0xE5, 0x92, 0xBE, 0x57, 0x75, 0x92,
        0x00, 0x76, 0x56, 0xE5, 0x83, 0x78, 0x56, 0xE5, 0x82, 0x7A, 0x56, 0xE5, 0x9F,
    ])
}

fn flash_read_end<T: UsbTransport>(debugger: &mut CcDebugger<T>) -> Result<()> {
    debugger.send_raw_data(&[
        0xCA, 0x57, 0x75, 0x9F, 0xD6, 0x57, 0x90, 0xC4, 0x57, 0x75, 0x92, 0xC2, 0x57, 0x75, 0xD0,
        0x90, 0x56, 0x74,
    ])
}

fn create_read_proc(count: usize) -> Vec<u8> {
    let mut proc = Vec::with_capacity(count * 9);
    for index in 0..count {
        proc.extend_from_slice(&[0x5E, 0x55, 0xE4]);
        let mut read_cmd = 0x4E;
        if (index + 1) % 64 == 0 || index + 1 == count {
            read_cmd |= 1;
        }
        proc.extend_from_slice(&[read_cmd, 0x55, 0x93, 0x5E, 0x55, 0xA3]);
    }
    proc
}

fn flash_read_near<T: UsbTransport>(
    debugger: &mut CcDebugger<T>,
    address: u32,
    size: usize,
) -> Result<Vec<u8>> {
    let mut data = Vec::with_capacity(size);
    debugger.send_raw_data(&[0xBE, 0x57, 0x90, (address >> 8) as u8, address as u8])?;
    let mut offset = 0;
    while offset < size {
        let count = (size - offset).min(128);
        debugger.send_raw_data(&create_read_proc(count))?;
        let chunk = debugger.bulk_in(count)?;
        if chunk.len() != count {
            return Err(Error::ShortTransfer {
                operation: "CC2530 Flash read",
                expected: count,
                actual: chunk.len(),
            });
        }
        data.extend_from_slice(&chunk);
        offset += count;
    }
    Ok(data)
}

fn flash_read<T: UsbTransport>(
    debugger: &mut CcDebugger<T>,
    mut offset: u32,
    mut size: usize,
) -> Result<Vec<u8>> {
    let mut all_data = Vec::with_capacity(size);
    let mut selected_bank = None;
    while size > 0 {
        let bank_offset = offset % FLASH_BANK_SIZE;
        let mut count = size.min(FLASH_BANK_SIZE as usize);
        let bank0 = offset / FLASH_BANK_SIZE;
        let bank1 = (offset + count as u32) / FLASH_BANK_SIZE;
        if bank0 != bank1 {
            count = (FLASH_BANK_SIZE - bank_offset) as usize;
        }
        if selected_bank != Some(bank0) {
            debugger.write_xdata(XREG_FMAP, &[bank0 as u8])?;
            selected_bank = Some(bank0);
        }
        all_data.extend_from_slice(&flash_read_near(
            debugger,
            bank_offset + XBANK_OFFSET,
            count,
        )?);
        size -= count;
        offset += count as u32;
    }
    Ok(all_data)
}

fn verify_image<T: UsbTransport>(debugger: &mut CcDebugger<T>, image: &[u8]) -> Result<()> {
    debugger.init_debug_interface()?;
    eprintln!("Verifying {} bytes by readback", image.len());
    flash_read_start(debugger)?;
    let verification = (|| {
        let mut offset = 0;
        while offset < image.len() {
            let count = (image.len() - offset).min(4096);
            let actual = flash_read(debugger, offset as u32, count)?;
            check_image_chunk(offset, &image[offset..offset + count], &actual)?;
            offset += count;
            eprintln!("Verify {}%", offset * 100 / image.len());
        }
        Ok(())
    })();
    let end_result = flash_read_end(debugger);
    match (verification, end_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn check_image_chunk(offset: usize, expected: &[u8], actual: &[u8]) -> Result<()> {
    if actual.len() != expected.len() {
        return Err(Error::ShortTransfer {
            operation: "CC2530 Flash verify readback",
            expected: expected.len(),
            actual: actual.len(),
        });
    }
    if let Some((index, (got, want))) = actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (got, want))| got != want)
    {
        return Err(Error::protocol(format!(
            "verify failed at 0x{:04X}: expected 0x{want:02X}, got 0x{got:02X}",
            offset + index
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{
        FLASH_BANK_SIZE, FlashOptions, check_image_chunk, create_read_proc,
        flash_connected_debugger, image_from_hex,
    };
    use crate::cc_debugger::{CcDebugger, Timing};
    use crate::error::{Error, Result};
    use crate::intel_hex::IntelHexImage;
    use crate::usb::UsbTransport;

    #[derive(Clone, Default)]
    struct UsbLog {
        control_outs: Arc<Mutex<Vec<(u8, u16, u16)>>>,
        bulk_outs: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    struct FakeFlashUsb {
        control_reads: VecDeque<Vec<u8>>,
        bulk_reads: VecDeque<Vec<u8>>,
        log: UsbLog,
    }

    impl UsbTransport for FakeFlashUsb {
        fn programmer(&self) -> &'static str {
            "CC Debugger"
        }

        fn control_out(&mut self, request: u8, value: u16, index: u16, _: &[u8]) -> Result<()> {
            self.log
                .control_outs
                .lock()
                .unwrap()
                .push((request, value, index));
            Ok(())
        }

        fn control_in(&mut self, _: u8, _: u16, _: u16, _: usize) -> Result<Vec<u8>> {
            self.control_reads
                .pop_front()
                .ok_or_else(|| Error::protocol("no fake control response"))
        }

        fn bulk_out(&mut self, data: &[u8]) -> Result<()> {
            self.log.bulk_outs.lock().unwrap().push(data.to_vec());
            Ok(())
        }

        fn bulk_in(&mut self, length: usize) -> Result<Vec<u8>> {
            Ok(self
                .bulk_reads
                .pop_front()
                .unwrap_or_else(|| vec![0; length]))
        }
    }

    fn device_info(chip_id: u16) -> Vec<u8> {
        let [low, high] = chip_id.to_le_bytes();
        vec![low, high, 1, 0, 2, 0]
    }

    fn zero_timing() -> Timing {
        Timing {
            reset_delay: Duration::ZERO,
            debug_settle: Duration::ZERO,
            erase_timeout: Duration::ZERO,
            erase_poll: Duration::ZERO,
            flash_timeout: Duration::ZERO,
            flash_poll: Duration::ZERO,
        }
    }

    #[test]
    fn pads_sparse_image_gaps_and_tail_with_erased_flash_value() {
        let image = IntelHexImage::from_text(":01001000AA45\n:00000001FF\n").unwrap();
        let padded = image_from_hex(&image, 1024).unwrap();
        assert_eq!(padded.len(), 1024);
        assert_eq!(padded[0x10], 0xAA);
        assert!(padded[..0x10].iter().all(|byte| *byte == 0xFF));
        assert!(padded[0x11..].iter().all(|byte| *byte == 0xFF));
    }

    #[test]
    fn rejects_non_cc2530_before_issuing_debugger_or_flash_commands() {
        let log = UsbLog::default();
        let fake = FakeFlashUsb {
            control_reads: VecDeque::from([device_info(0x2531)]),
            bulk_reads: VecDeque::new(),
            log: log.clone(),
        };
        let mut debugger = CcDebugger::with_timing(fake, zero_timing());

        let error = flash_connected_debugger(
            &mut debugger,
            &[0xFF; 1024],
            FlashOptions {
                erase: true,
                write: true,
                verify: true,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("expected CC2530"));
        assert!(log.control_outs.lock().unwrap().is_empty());
        assert!(log.bulk_outs.lock().unwrap().is_empty());
    }

    #[test]
    fn attempts_normal_reset_after_flash_operation_failure() {
        let log = UsbLog::default();
        let fake = FakeFlashUsb {
            control_reads: VecDeque::from([device_info(0x2530), device_info(0x2530)]),
            bulk_reads: VecDeque::from([vec![0]]),
            log: log.clone(),
        };
        let mut debugger = CcDebugger::with_timing(fake, zero_timing());

        let error = flash_connected_debugger(
            &mut debugger,
            &[0xFF; 1024],
            FlashOptions {
                erase: true,
                write: false,
                verify: false,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("chip erase timed out"));
        let control_outs = log.control_outs.lock().unwrap();
        assert!(control_outs.contains(&(0xC9, 0, 1)));
        assert_eq!(control_outs.last(), Some(&(0xC9, 0, 0)));
    }

    #[test]
    fn rejects_zero_block_size_and_non_aligned_address_limit_padding() {
        let image = IntelHexImage::from_text(":0100000001FE\n:00000001FF\n").unwrap();
        assert!(image_from_hex(&image, 0).is_err());
    }

    #[test]
    fn reports_first_readback_mismatch_with_absolute_offset() {
        let error = check_image_chunk(0x1234, &[1, 2, 3], &[1, 9, 3]).unwrap_err();
        assert!(matches!(error, Error::Protocol(message) if message.contains("0x1235")));
        assert!(matches!(
            check_image_chunk(0, &[1, 2], &[1]),
            Err(Error::ShortTransfer { .. })
        ));
    }

    #[test]
    fn mocked_usb_readback_mismatch_fails_flash_verification() {
        use std::collections::VecDeque;
        use std::time::Duration;

        use super::verify_image;
        use crate::cc_debugger::{CcDebugger, Timing};
        use crate::error::Result;
        use crate::usb::UsbTransport;

        #[derive(Default)]
        struct FakeUsb {
            control_reads: VecDeque<Vec<u8>>,
            bulk_reads: VecDeque<Vec<u8>>,
        }

        impl UsbTransport for FakeUsb {
            fn programmer(&self) -> &'static str {
                "CC Debugger"
            }
            fn control_out(&mut self, _: u8, _: u16, _: u16, _: &[u8]) -> Result<()> {
                Ok(())
            }
            fn control_in(&mut self, _: u8, _: u16, _: u16, _: usize) -> Result<Vec<u8>> {
                Ok(self.control_reads.pop_front().unwrap_or_default())
            }
            fn bulk_out(&mut self, _: &[u8]) -> Result<()> {
                Ok(())
            }
            fn bulk_in(&mut self, length: usize) -> Result<Vec<u8>> {
                Ok(self
                    .bulk_reads
                    .pop_front()
                    .unwrap_or_else(|| vec![0; length]))
            }
        }

        let mut bulk_reads = VecDeque::from([vec![0]]); // debug status during interface initialization
        bulk_reads.extend((0..8).map(|_| vec![0; 128]));
        let fake = FakeUsb {
            control_reads: VecDeque::from([vec![0x30, 0x25, 1, 0, 2, 0], vec![0]]),
            bulk_reads,
        };
        let timing = Timing {
            reset_delay: Duration::ZERO,
            debug_settle: Duration::ZERO,
            erase_timeout: Duration::ZERO,
            erase_poll: Duration::ZERO,
            flash_timeout: Duration::from_millis(1),
            flash_poll: Duration::ZERO,
        };
        let mut debugger = CcDebugger::with_timing(fake, timing);
        let error = verify_image(&mut debugger, &vec![0xAA; 1024]).unwrap_err();
        assert!(matches!(
            error,
            Error::Protocol(message) if message.contains("verify failed at 0x0000")
        ));
    }

    #[test]
    fn bank_size_and_read_program_chunk_commands_are_stable() {
        assert_eq!(FLASH_BANK_SIZE, 0x8000);
        let proc = create_read_proc(65);
        assert_eq!(proc.len(), 65 * 9);
        assert_eq!(proc[63 * 9 + 3], 0x4F); // every 64th byte asks the debugger to flush data
        assert_eq!(proc[64 * 9 + 3], 0x4F); // final byte also flushes
    }
}
