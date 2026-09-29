//! TI CC Debugger USB protocol and CC2530 debug-interface access.
//!
//! Protocol values and command framing are ported from Breezio-neo's xzg_cli.

use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::usb::{RusbTransport, UsbTransport};

const REQ_GET_STATE: u8 = 0xC0;
const REQ_PREPARE_DEBUG_MODE: u8 = 0xC5;
const REQ_SET_CHIP_INFO: u8 = 0xC8;
const REQ_RESET: u8 = 0xC9;

const CMD_EXEC_3BYTE: u8 = 0xBE;
const CMD_EXEC_2BYTE: u8 = 0x8E;
const CMD_EXEC_1BYTE: u8 = 0x5E;
const CMD_EXEC_1BYTE_READ: u8 = 0x4E;

const ASM_MOV_DPTR_IMM16: u8 = 0x90;
const ASM_MOV_A_IMM8: u8 = 0x74;
const ASM_MOVX_A_AT_DPTR: u8 = 0xE0;
const ASM_MOVX_AT_DPTR_A: u8 = 0xF0;
const ASM_INC_DPTR: u8 = 0xA3;

const DEBUG_CMD_WR_CONFIG: u8 = 0x1D;
const DEBUG_CMD_READ_STATUS: u8 = 0x34;
const DEBUG_CMD_DEBUG_INSTR_1: u8 = 0x55;
const DEBUG_CMD_DEBUG_INSTR_2: u8 = 0x56;
const DEBUG_CMD_DEBUG_INSTR_3: u8 = 0x57;

const WRAPPER_DEBUG_EXEC: u8 = 0x1C;
const WRAPPER_DEBUG_EXEC_READ: u8 = 0x1F;
const WRAPPER_DEBUG_EXEC_ARG: u8 = 0x4C;

const CMD_HEADER: &[u8] = &[
    0x40, 0x55, 0x00, 0x72, 0x56, 0xE5, 0x92, 0xBE, 0x57, 0x75, 0x92, 0x00, 0x74, 0x56, 0xE5, 0x83,
    0x76, 0x56, 0xE5, 0x82,
];
const CMD_FOOTER: &[u8] = &[0xD4, 0x57, 0x90, 0xC2, 0x57, 0x75, 0x92, 0x90, 0x56, 0x74];

pub const XREG_FCTL: u16 = 0x6270;
pub const XREG_FADDRL: u16 = 0x6271;
pub const XREG_FADDRH: u16 = 0x6272;
pub const XREG_DMAARM: u16 = 0x70D6;
pub const XREG_FMAP: u16 = 0x709F;
pub const ADDR_IEEE_PRIMARY: u16 = 0x780C;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TiProbeInfo {
    pub programmer: &'static str,
    pub chip_id: u16,
    pub fw_version: u16,
    pub fw_revision: u16,
    pub ieee_address: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub reset_delay: Duration,
    pub debug_settle: Duration,
    pub erase_timeout: Duration,
    pub erase_poll: Duration,
    pub flash_timeout: Duration,
    pub flash_poll: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            reset_delay: Duration::from_millis(100),
            debug_settle: Duration::from_secs(1),
            erase_timeout: Duration::from_secs(8),
            erase_poll: Duration::from_millis(100),
            flash_timeout: Duration::from_secs(5),
            flash_poll: Duration::from_millis(10),
        }
    }
}

pub struct CcDebugger<T: UsbTransport> {
    transport: T,
    initialized: bool,
    timing: Timing,
}

impl<T: UsbTransport> CcDebugger<T> {
    pub fn new(transport: T) -> Self {
        Self::with_timing(transport, Timing::default())
    }

    pub fn with_timing(transport: T, timing: Timing) -> Self {
        Self {
            transport,
            initialized: false,
            timing,
        }
    }

    pub fn programmer(&self) -> &'static str {
        self.transport.programmer()
    }

    pub fn control_in(
        &mut self,
        request: u8,
        value: u16,
        index: u16,
        length: usize,
    ) -> Result<Vec<u8>> {
        self.transport.control_in(request, value, index, length)
    }

    pub fn control_out(&mut self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()> {
        self.transport.control_out(request, value, index, data)
    }

    pub fn bulk_in(&mut self, length: usize) -> Result<Vec<u8>> {
        self.transport.bulk_in(length)
    }

    pub fn bulk_in_exact(&mut self, length: usize, operation: &'static str) -> Result<Vec<u8>> {
        let data = self.bulk_in(length)?;
        if data.len() != length {
            return Err(Error::ShortTransfer {
                operation,
                expected: length,
                actual: data.len(),
            });
        }
        Ok(data)
    }

    pub fn bulk_out(&mut self, data: &[u8]) -> Result<()> {
        self.transport.bulk_out(data)
    }

    pub fn get_device_info(&mut self) -> Result<TiProbeInfo> {
        let data = self.control_in(REQ_GET_STATE, 0, 0, 8)?;
        if data.len() < 6 {
            return Err(Error::protocol(format!(
                "short GET_STATE response: {}",
                hex_bytes(&data)
            )));
        }
        Ok(TiProbeInfo {
            programmer: self.programmer(),
            chip_id: u16::from_le_bytes([data[0], data[1]]),
            fw_version: u16::from_le_bytes([data[2], data[3]]),
            fw_revision: u16::from_le_bytes([data[4], data[5]]),
            ieee_address: None,
        })
    }

    pub fn prepare_debug_mode(&mut self) -> Result<()> {
        self.control_out(REQ_PREPARE_DEBUG_MODE, 0, 0, &[])?;
        let info = self.get_device_info()?;
        let mut command = [b' '; 0x30];
        let chip_name = format!("CC{:04X}", info.chip_id);
        command[..chip_name.len()].copy_from_slice(chip_name.as_bytes());
        command[0x10..0x14].copy_from_slice(b"DID:");
        let did = format!("{:04X}", info.fw_version);
        command[0x15..0x15 + did.len()].copy_from_slice(did.as_bytes());
        self.control_out(REQ_SET_CHIP_INFO, 1, 0, &command)
    }

    pub fn reset(&mut self, debug_mode: bool) -> Result<()> {
        self.control_out(REQ_RESET, 0, u16::from(debug_mode), &[])?;
        std::thread::sleep(self.timing.reset_delay);
        self.initialized = false;
        Ok(())
    }

    pub fn init_debug_interface(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        eprintln!("Initializing debug interface");
        self.prepare_debug_mode()?;
        self.reset(true)?;
        std::thread::sleep(self.timing.debug_settle);
        self.send_debug_instructions(&[WRAPPER_DEBUG_EXEC_READ, DEBUG_CMD_READ_STATUS])?;
        let status = self.bulk_in_exact(1, "CC Debugger status read")?;
        eprintln!("Debug status: 0x{:02x}", status[0]);
        self.send_debug_instructions(&[WRAPPER_DEBUG_EXEC_ARG, DEBUG_CMD_WR_CONFIG, 0x22])?;
        self.initialized = true;
        Ok(())
    }

    pub fn send_debug_instructions(&mut self, instructions: &[u8]) -> Result<()> {
        self.bulk_out(instructions)
    }

    pub fn send_raw_data(&mut self, data: &[u8]) -> Result<()> {
        self.bulk_out(data)
    }

    pub fn read_xdata(&mut self, address: u16, length: usize) -> Result<Vec<u8>> {
        if u32::from(address) + length as u32 > 0x1_0000 {
            return Err(Error::protocol("XDATA read exceeds 0xFFFF"));
        }
        self.init_debug_interface()?;
        let mut result = Vec::with_capacity(length);
        for offset in (0..length).step_by(512) {
            let current = (length - offset).min(512);
            let current_address = u32::from(address) + offset as u32;
            let mut command =
                Vec::with_capacity(CMD_HEADER.len() + current * 6 + CMD_FOOTER.len() + 8);
            command.extend_from_slice(CMD_HEADER);
            command.extend_from_slice(&[
                CMD_EXEC_3BYTE,
                DEBUG_CMD_DEBUG_INSTR_3,
                ASM_MOV_DPTR_IMM16,
                (current_address >> 8) as u8,
                current_address as u8,
            ]);
            for index in 0..current {
                let mut read_cmd = CMD_EXEC_1BYTE_READ;
                if index + 1 == current || (index + 1) % 64 == 0 {
                    read_cmd |= 1;
                }
                command.extend_from_slice(&[
                    read_cmd,
                    DEBUG_CMD_DEBUG_INSTR_1,
                    ASM_MOVX_A_AT_DPTR,
                    CMD_EXEC_1BYTE,
                    DEBUG_CMD_DEBUG_INSTR_1,
                    ASM_INC_DPTR,
                ]);
            }
            command.extend_from_slice(CMD_FOOTER);
            self.send_debug_instructions(&command)?;
            let data = self.bulk_in(current)?;
            if data.len() != current {
                return Err(Error::ShortTransfer {
                    operation: "CC2530 XDATA read",
                    expected: current,
                    actual: data.len(),
                });
            }
            result.extend_from_slice(&data);
        }
        Ok(result)
    }

    pub fn write_xdata(&mut self, address: u16, data: &[u8]) -> Result<()> {
        if u32::from(address) + data.len() as u32 > 0x1_0000 {
            return Err(Error::protocol("XDATA write exceeds 0xFFFF"));
        }
        self.init_debug_interface()?;
        for (offset, chunk) in data.chunks(512).enumerate() {
            let current_address = u32::from(address) + (offset * 512) as u32;
            let mut command =
                Vec::with_capacity(CMD_HEADER.len() + chunk.len() * 10 + CMD_FOOTER.len() + 8);
            command.extend_from_slice(CMD_HEADER);
            command.extend_from_slice(&[
                CMD_EXEC_3BYTE,
                DEBUG_CMD_DEBUG_INSTR_3,
                ASM_MOV_DPTR_IMM16,
                (current_address >> 8) as u8,
                current_address as u8,
            ]);
            for byte in chunk {
                command.extend_from_slice(&[
                    CMD_EXEC_2BYTE,
                    DEBUG_CMD_DEBUG_INSTR_2,
                    ASM_MOV_A_IMM8,
                    *byte,
                    CMD_EXEC_1BYTE,
                    DEBUG_CMD_DEBUG_INSTR_1,
                    ASM_MOVX_AT_DPTR_A,
                    CMD_EXEC_1BYTE,
                    DEBUG_CMD_DEBUG_INSTR_1,
                    ASM_INC_DPTR,
                ]);
            }
            command.extend_from_slice(CMD_FOOTER);
            self.send_debug_instructions(&command)?;
        }
        Ok(())
    }

    pub fn read_ieee_address(&mut self) -> Result<String> {
        let raw = self.read_xdata(ADDR_IEEE_PRIMARY, 8)?;
        Ok(raw
            .iter()
            .rev()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":"))
    }

    pub fn probe(&mut self, read_ieee: bool) -> Result<TiProbeInfo> {
        let mut info = self.get_device_info()?;
        if read_ieee {
            info.ieee_address = Some(self.read_ieee_address()?);
            // IEEE reads enter debug mode; leave a probe-only target running normally.
            self.reset(false)?;
        }
        Ok(info)
    }

    pub fn chip_erase(&mut self) -> Result<()> {
        self.init_debug_interface()?;
        eprintln!("Erasing chip");
        self.send_debug_instructions(&[WRAPPER_DEBUG_EXEC, 0x14])?;
        let deadline = Instant::now() + self.timing.erase_timeout;
        while Instant::now() < deadline {
            self.send_debug_instructions(&[WRAPPER_DEBUG_EXEC_READ, DEBUG_CMD_READ_STATUS])?;
            let status = self.bulk_in_exact(1, "CC Debugger erase status read")?;
            if status[0] & 0x80 == 0 {
                eprintln!("Erase complete");
                self.initialized = false;
                return Ok(());
            }
            std::thread::sleep(self.timing.erase_poll);
        }
        Err(Error::protocol("chip erase timed out"))
    }

    pub(crate) fn timing(&self) -> Timing {
        self.timing
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn probe_ti_debugger(read_ieee: bool) -> Result<TiProbeInfo> {
    let transport = RusbTransport::connect()?;
    CcDebugger::new(transport).probe(read_ieee)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::time::Duration;

    use super::{CcDebugger, Timing};
    use crate::error::{Error, Result};
    use crate::usb::UsbTransport;

    #[derive(Default)]
    struct FakeUsb {
        control_reads: VecDeque<Vec<u8>>,
        bulk_reads: VecDeque<Vec<u8>>,
        bulk_default: Option<Vec<u8>>,
        writes: Vec<Vec<u8>>,
    }

    impl UsbTransport for FakeUsb {
        fn programmer(&self) -> &'static str {
            "CC Debugger"
        }

        fn control_out(
            &mut self,
            _request: u8,
            _value: u16,
            _index: u16,
            _data: &[u8],
        ) -> Result<()> {
            Ok(())
        }

        fn control_in(
            &mut self,
            _request: u8,
            _value: u16,
            _index: u16,
            _length: usize,
        ) -> Result<Vec<u8>> {
            self.control_reads
                .pop_front()
                .ok_or_else(|| Error::protocol("no fake control response"))
        }

        fn bulk_out(&mut self, data: &[u8]) -> Result<()> {
            self.writes.push(data.to_vec());
            Ok(())
        }

        fn bulk_in(&mut self, _length: usize) -> Result<Vec<u8>> {
            self.bulk_reads
                .pop_front()
                .or_else(|| self.bulk_default.clone())
                .ok_or_else(|| Error::protocol("no fake bulk response"))
        }
    }

    fn fast_timing() -> Timing {
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
    fn reports_short_device_state_as_protocol_error() {
        let fake = FakeUsb {
            control_reads: VecDeque::from([vec![0x30, 0x25]]),
            ..FakeUsb::default()
        };
        let error = CcDebugger::with_timing(fake, fast_timing())
            .get_device_info()
            .unwrap_err();
        assert!(error.to_string().contains("short GET_STATE"));
    }

    #[test]
    fn erase_timeout_is_reported_when_the_chip_stays_busy() {
        let fake = FakeUsb {
            control_reads: VecDeque::from([vec![0x30, 0x25, 1, 0, 2, 0]]),
            bulk_reads: VecDeque::from([vec![0]]),
            bulk_default: Some(vec![0x80]),
            ..FakeUsb::default()
        };
        let timing = Timing {
            erase_timeout: Duration::from_millis(2),
            ..fast_timing()
        };
        let error = CcDebugger::with_timing(fake, timing)
            .chip_erase()
            .unwrap_err();
        assert!(error.to_string().contains("chip erase timed out"));
    }

    #[test]
    fn reports_get_state_control_transfer_failure() {
        struct FailingUsb;
        impl UsbTransport for FailingUsb {
            fn programmer(&self) -> &'static str {
                "CC Debugger"
            }
            fn control_out(&mut self, _: u8, _: u16, _: u16, _: &[u8]) -> Result<()> {
                Ok(())
            }
            fn control_in(&mut self, _: u8, _: u16, _: u16, _: usize) -> Result<Vec<u8>> {
                Err(Error::ShortTransfer {
                    operation: "USB control IN",
                    expected: 8,
                    actual: 4,
                })
            }
            fn bulk_out(&mut self, _: &[u8]) -> Result<()> {
                Ok(())
            }
            fn bulk_in(&mut self, _: usize) -> Result<Vec<u8>> {
                Ok(vec![])
            }
        }
        assert!(matches!(
            CcDebugger::with_timing(FailingUsb, fast_timing()).get_device_info(),
            Err(Error::ShortTransfer { .. })
        ));
    }

    #[test]
    fn rejects_short_debug_status_transfer_instead_of_indexing_empty_data() {
        let fake = FakeUsb {
            control_reads: VecDeque::from([vec![0x30, 0x25, 1, 0, 2, 0]]),
            bulk_reads: VecDeque::from([vec![]]),
            ..FakeUsb::default()
        };
        let error = CcDebugger::with_timing(fake, fast_timing())
            .init_debug_interface()
            .unwrap_err();
        assert!(matches!(error, Error::ShortTransfer { .. }));
    }
}
