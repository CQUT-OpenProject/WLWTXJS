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
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate an Intel HEX firmware image.
    CheckHex { firmware: PathBuf },
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

fn run(cli: Cli) -> Result<()> {
    match cli.command {
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
        Command::ProbeTi { no_ieee } => {
            let info = cc_debugger::probe_ti_debugger(!no_ieee)?;
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
