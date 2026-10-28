//! Rust port of the native CC Debugger workflow in Breezio-neo's xzg_cli.
//! The implementation is rewritten here; see THIRD_PARTY_NOTICES.md.

mod cc2530;
mod cc_debugger;
mod error;
mod intel_hex;
mod usb;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use error::Result;
use intel_hex::IntelHexImage;
use serde_json::json;

#[derive(Debug, Parser)]
#[command(name = "xzg", version, about = "CC2530 programming helper")]
struct Cli {
    /// Select a debugger by USB bus:address when several are connected.
    #[arg(long, global = true, value_parser = parse_device_selector)]
    device: Option<usb::DeviceSelector>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate an Intel HEX firmware image.
    CheckHex { firmware: PathBuf },
    /// List connected TI CC Debugger / SmartRF04EB devices.
    ListTi,
    /// Probe a TI CC Debugger or compatible SmartRF04EB.
    ProbeTi {
        /// Skip reading the target's factory IEEE address.
        #[arg(long)]
        no_ieee: bool,
    },
    /// Erase, program, read back, and reset a CC2530 target.
    FlashTi {
        firmware: PathBuf,
        /// Do not erase the target before programming.
        #[arg(long)]
        no_erase: bool,
        /// Do not write firmware to the target.
        #[arg(long)]
        no_write: bool,
        /// Do not read back and verify firmware.
        #[arg(long)]
        no_verify: bool,
    },
}

fn parse_device_selector(value: &str) -> std::result::Result<usb::DeviceSelector, String> {
    let (bus, address) = value
        .split_once(':')
        .ok_or_else(|| "expected USB bus and address as BUS:ADDRESS".to_string())?;
    let bus = bus
        .parse::<u8>()
        .map_err(|_| "USB bus must be an integer from 0 to 255".to_string())?;
    let address = address
        .parse::<u8>()
        .map_err(|_| "USB address must be an integer from 0 to 255".to_string())?;
    Ok(usb::DeviceSelector { bus, address })
}

fn run(cli: Cli) -> Result<()> {
    let Cli { device, command } = cli;
    match command {
        Command::CheckHex { firmware } => {
            let image = IntelHexImage::from_file(&firmware)?;
            let summary = json!({
                "sections": image.sections().len(),
                "image_size": image.size(),
                "lowest_address": format!("0x{:05X}", image.lowest_address()),
                "highest_address": format!("0x{:05X}", image.highest_address()),
            });
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::ListTi => {
            let devices: Vec<_> = usb::list_supported_devices()?
                .into_iter()
                .map(|info| {
                    json!({
                        "programmer": info.programmer,
                        "bus": info.bus,
                        "address": info.address,
                        "selector": format!("{}:{}", info.bus, info.address),
                        "vendor_id": format!("0x{:04X}", info.vendor_id),
                        "product_id": format!("0x{:04X}", info.product_id),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&devices)?);
        }
        Command::ProbeTi { no_ieee } => {
            let info = cc_debugger::probe_ti_debugger(!no_ieee, device)?;
            let output = json!({
                "programmer": info.programmer,
                "chip_id": format!("0x{:04X}", info.chip_id),
                "fw_version": format!("0x{:04X}", info.fw_version),
                "fw_revision": format!("0x{:04X}", info.fw_revision),
                "ieee_address": info.ieee_address,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Command::FlashTi {
            firmware,
            no_erase,
            no_write,
            no_verify,
        } => cc2530::flash_ti_debugger(
            &firmware,
            cc2530::FlashOptions {
                erase: !no_erase,
                write: !no_write,
                verify: !no_verify,
            },
            device,
        )?,
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn cli_exposes_migrated_commands_and_flags() {
        assert!(matches!(
            Cli::try_parse_from(["xzg", "check-hex", "image.hex"])
                .unwrap()
                .command,
            Command::CheckHex { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["xzg", "probe-ti", "--no-ieee"])
                .unwrap()
                .command,
            Command::ProbeTi { no_ieee: true }
        ));
        assert!(matches!(
            Cli::try_parse_from([
                "xzg",
                "flash-ti",
                "image.hex",
                "--no-erase",
                "--no-write",
                "--no-verify"
            ])
            .unwrap()
            .command,
            Command::FlashTi {
                no_erase: true,
                no_write: true,
                no_verify: true,
                ..
            }
        ));
    }

    #[test]
    fn cli_accepts_device_selector_before_or_after_subcommand() {
        let before = Cli::try_parse_from(["xzg", "--device", "1:5", "probe-ti"]).unwrap();
        assert_eq!(before.device.map(|d| (d.bus, d.address)), Some((1, 5)));

        let after = Cli::try_parse_from(["xzg", "probe-ti", "--device", "2:7"]).unwrap();
        assert_eq!(after.device.map(|d| (d.bus, d.address)), Some((2, 7)));

        assert!(Cli::try_parse_from(["xzg", "probe-ti", "--device", "1"]).is_err());
    }

    #[test]
    fn cli_exposes_list_ti_command() {
        assert!(matches!(
            Cli::try_parse_from(["xzg", "list-ti"]).unwrap().command,
            Command::ListTi
        ));
    }

    #[test]
    fn cli_rejects_removed_cc_tool_and_bridge_commands() {
        assert!(Cli::try_parse_from(["xzg", "flash-cc2530", "image.hex"]).is_err());
        assert!(Cli::try_parse_from(["xzg", "bridge-discover"]).is_err());
    }

    #[test]
    fn invalid_or_missing_firmware_returns_an_error_for_nonzero_exit() {
        let cli =
            Cli::try_parse_from(["xzg", "check-hex", "/definitely/not/a/firmware.hex"]).unwrap();
        assert!(super::run(cli).is_err());
    }
}
