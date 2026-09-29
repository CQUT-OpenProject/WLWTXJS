//! Intel HEX parser and CC2530 address-range validation.
//!
//! This is a Rust rewrite of the corresponding Breezio-neo xzg_cli parser.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

pub const CC2530_MAX_ADDRESS: u32 = 0xFFFF;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HexSection {
    pub address: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntelHexImage {
    sections: Vec<HexSection>,
}

impl IntelHexImage {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_text(&content)
    }

    pub fn from_text(content: &str) -> Result<Self> {
        let mut bytes = BTreeMap::<u16, u8>::new();
        let mut address_prefix = 0_u32;
        let mut saw_eof = false;

        for (line_index, raw_line) in content.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if saw_eof {
                return Err(Error::hex(format!(
                    "line {line_number}: non-empty content after EOF record"
                )));
            }
            if !line.is_ascii() || !line.starts_with(':') {
                return Err(Error::hex(format!(
                    "line {line_number}: expected an ASCII Intel HEX record beginning with ':'"
                )));
            }
            if line.len() < 11 || line.len() % 2 != 1 {
                return Err(Error::hex(format!(
                    "line {line_number}: invalid or odd record length"
                )));
            }

            let byte_count = parse_hex_byte(&line[1..3], line_number)? as usize;
            let expected_len = 11 + byte_count * 2;
            if line.len() != expected_len {
                return Err(Error::hex(format!(
                    "line {line_number}: expected length {expected_len}, got {}",
                    line.len()
                )));
            }

            let address = parse_hex_u16(&line[3..7], line_number)?;
            let record_type = parse_hex_byte(&line[7..9], line_number)?;
            let mut data = Vec::with_capacity(byte_count);
            for index in 0..byte_count {
                let start = 9 + index * 2;
                data.push(parse_hex_byte(&line[start..start + 2], line_number)?);
            }
            let checksum_index = 9 + byte_count * 2;
            let checksum = parse_hex_byte(&line[checksum_index..checksum_index + 2], line_number)?;
            let sum = (byte_count as u8)
                .wrapping_add((address >> 8) as u8)
                .wrapping_add(address as u8)
                .wrapping_add(record_type)
                .wrapping_add(data.iter().copied().fold(0_u8, u8::wrapping_add))
                .wrapping_add(checksum);
            if sum != 0 {
                return Err(Error::hex(format!("line {line_number}: checksum mismatch")));
            }

            match record_type {
                0x00 => {
                    let start = address_prefix + u32::from(address);
                    let end = start + data.len() as u32;
                    if end > CC2530_MAX_ADDRESS + 1 {
                        return Err(Error::hex(format!(
                            "line {line_number}: data address range 0x{start:05X}..0x{:05X} exceeds 0x{CC2530_MAX_ADDRESS:05X}",
                            end.saturating_sub(1)
                        )));
                    }
                    for (offset, byte) in data.into_iter().enumerate() {
                        let byte_address = (start + offset as u32) as u16;
                        if bytes.insert(byte_address, byte).is_some() {
                            return Err(Error::hex(format!(
                                "line {line_number}: overlapping data at address 0x{byte_address:04X}"
                            )));
                        }
                    }
                }
                0x01 => {
                    if address != 0 || !data.is_empty() {
                        return Err(Error::hex(format!(
                            "line {line_number}: EOF record must have address 0 and no data"
                        )));
                    }
                    saw_eof = true;
                }
                0x02 => {
                    if data.len() != 2 {
                        return Err(Error::hex(format!(
                            "line {line_number}: extended segment address must contain 2 bytes"
                        )));
                    }
                    address_prefix = u32::from(u16::from_be_bytes([data[0], data[1]])) << 4;
                }
                0x04 => {
                    if data.len() != 2 {
                        return Err(Error::hex(format!(
                            "line {line_number}: extended linear address must contain 2 bytes"
                        )));
                    }
                    address_prefix = u32::from(u16::from_be_bytes([data[0], data[1]])) << 16;
                }
                other => {
                    return Err(Error::hex(format!(
                        "line {line_number}: unsupported record type 0x{other:02X}"
                    )));
                }
            }
        }

        if !saw_eof {
            return Err(Error::hex("missing EOF record"));
        }
        if bytes.is_empty() {
            return Err(Error::hex("HEX file contains no data"));
        }

        let mut sections = Vec::new();
        let mut current_address = None;
        let mut previous_address = 0_u16;
        let mut current_data = Vec::new();
        for (address, byte) in bytes {
            if current_address
                .is_some_and(|_| u32::from(previous_address) + 1 != u32::from(address))
            {
                sections.push(HexSection {
                    address: current_address.take().expect("section has a start"),
                    data: std::mem::take(&mut current_data),
                });
            }
            if current_address.is_none() {
                current_address = Some(address);
            }
            current_data.push(byte);
            previous_address = address;
        }
        sections.push(HexSection {
            address: current_address.expect("non-empty image has a section"),
            data: current_data,
        });

        Ok(Self { sections })
    }

    pub fn sections(&self) -> &[HexSection] {
        &self.sections
    }

    pub fn lowest_address(&self) -> u32 {
        self.sections
            .iter()
            .map(|section| u32::from(section.address))
            .min()
            .expect("validated HEX images are non-empty")
    }

    pub fn highest_address(&self) -> u32 {
        self.sections
            .iter()
            .map(|section| u32::from(section.address) + section.data.len() as u32 - 1)
            .max()
            .expect("validated HEX images are non-empty")
    }

    pub fn size(&self) -> usize {
        self.sections.iter().map(|section| section.data.len()).sum()
    }

    pub fn validate_range(&self, max_address: u32) -> Result<()> {
        if self.sections.is_empty()
            || self.lowest_address() > max_address
            || self.highest_address() > max_address
        {
            return Err(Error::hex(format!(
                "HEX address range 0x{:05X}..0x{:05X} exceeds 0x{max_address:05X}",
                self.lowest_address(),
                self.highest_address()
            )));
        }
        Ok(())
    }
}

fn parse_hex_byte(text: &str, line_number: usize) -> Result<u8> {
    u8::from_str_radix(text, 16).map_err(|_| {
        Error::hex(format!(
            "line {line_number}: invalid hexadecimal byte '{text}'"
        ))
    })
}

fn parse_hex_u16(text: &str, line_number: usize) -> Result<u16> {
    u16::from_str_radix(text, 16).map_err(|_| {
        Error::hex(format!(
            "line {line_number}: invalid hexadecimal address '{text}'"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::{CC2530_MAX_ADDRESS, IntelHexImage};
    use crate::cc2530::image_from_hex;

    const SAMPLE: &str = ":100000000102030405060708090A0B0C0D0E0F1068\n:00000001FF\n";

    fn record(address: u16, kind: u8, data: &[u8]) -> String {
        let mut bytes = vec![data.len() as u8, (address >> 8) as u8, address as u8, kind];
        bytes.extend_from_slice(data);
        let checksum = (0_u8).wrapping_sub(bytes.iter().copied().fold(0_u8, u8::wrapping_add));
        bytes.push(checksum);
        format!(
            ":{}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        )
    }

    #[test]
    fn parses_and_merges_contiguous_data_records() {
        let content = format!("{}\n{}\n", record(0, 0, &[1, 2]), record(2, 0, &[3, 4]));
        let content = format!("{content}{}\n", record(0, 1, &[]));
        let image = IntelHexImage::from_text(&content).unwrap();
        assert_eq!(image.sections().len(), 1);
        assert_eq!(image.size(), 4);
        assert_eq!(image.sections()[0].data, [1, 2, 3, 4]);
    }

    #[test]
    fn rejects_bad_checksum_and_record_length() {
        assert!(
            IntelHexImage::from_text(":100000000102030405060708090A0B0C0D0E0F1069\n:00000001FF\n")
                .is_err()
        );
        assert!(IntelHexImage::from_text(":0200000001FF\n:00000001FF\n").is_err());
    }

    #[test]
    fn requires_eof_and_at_least_one_data_byte() {
        assert!(IntelHexImage::from_text(&record(0, 0, &[1])).is_err());
        assert!(IntelHexImage::from_text(&record(0, 1, &[])).is_err());
    }

    #[test]
    fn supports_extended_segment_and_linear_address_records() {
        let segment = format!(
            "{}\n{}\n{}\n",
            record(0, 2, &[0, 1]),
            record(0, 0, &[0xAA]),
            record(0, 1, &[])
        );
        let image = IntelHexImage::from_text(&segment).unwrap();
        assert_eq!(image.lowest_address(), 0x10);

        let linear = format!(
            "{}\n{}\n{}\n",
            record(0, 4, &[0, 0]),
            record(0, 0, &[0xBB]),
            record(0, 1, &[])
        );
        let image = IntelHexImage::from_text(&linear).unwrap();
        assert_eq!(image.sections()[0].data, [0xBB]);
    }

    #[test]
    fn rejects_unsupported_records_overlaps_and_out_of_range_addresses() {
        let unsupported = format!("{}\n{}\n", record(0, 5, &[0, 0, 0, 0]), record(0, 1, &[]));
        assert!(IntelHexImage::from_text(&unsupported).is_err());
        let overlap = format!(
            "{}\n{}\n{}\n",
            record(0, 0, &[1, 2]),
            record(1, 0, &[3]),
            record(0, 1, &[])
        );
        assert!(IntelHexImage::from_text(&overlap).is_err());
        let overflow = format!(
            "{}\n{}\n{}\n",
            record(0, 4, &[0, 1]),
            record(0, 0, &[1]),
            record(0, 1, &[])
        );
        assert!(IntelHexImage::from_text(&overflow).is_err());
        let upper_edge = format!("{}\n{}\n", record(0xFFFF, 0, &[0xA5]), record(0, 1, &[]));
        let image = IntelHexImage::from_text(&upper_edge).unwrap();
        image.validate_range(CC2530_MAX_ADDRESS).unwrap();
        assert_eq!(image.highest_address(), 0xFFFF);
    }

    #[test]
    fn rejects_nonempty_content_after_eof() {
        let content = format!(
            "{}\n{}\n{}\n",
            record(0, 0, &[1]),
            record(0, 1, &[]),
            record(1, 0, &[2])
        );
        assert!(IntelHexImage::from_text(&content).is_err());
    }

    #[test]
    fn image_is_padded_with_ff_to_one_kib_blocks() {
        let image = IntelHexImage::from_text(SAMPLE).unwrap();
        let padded = image_from_hex(&image, 1024).unwrap();
        assert_eq!(padded.len(), 1024);
        assert_eq!(&padded[..16], &(1_u8..=16).collect::<Vec<_>>());
        assert!(padded[16..].iter().all(|byte| *byte == 0xFF));

        let top = IntelHexImage::from_text(&format!(
            "{}\n{}\n",
            record(0xFFFF, 0, &[7]),
            record(0, 1, &[])
        ))
        .unwrap();
        assert_eq!(image_from_hex(&top, 1024).unwrap().len(), 65536);
    }
}
