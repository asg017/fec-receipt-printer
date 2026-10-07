//! The `fec-receipt` CLI.

use std::{io::IsTerminal, path::PathBuf};

use clap::{Parser, Subcommand};
use fec_receipt::{DEFAULT_SAMPLES, escpos, forms, preview, receipt, serve, source, usb};

#[derive(Parser)]
#[command(about = "Print FEC filing covers on a receipt printer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a filing's receipt.
    Print {
        /// Filing ID (1926068 or FEC-1926068), URL, or path to a .fec file.
        filing: String,
        /// Add the detailed summary / every item / the whole text.
        #[arg(long)]
        full: bool,
        /// Write the ESC/POS bytes to this file instead of printing.
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
    /// Show a filing's receipt in the terminal.
    Preview {
        filing: String,
        #[arg(long)]
        full: bool,
        /// Write a standalone HTML preview to this file instead.
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Run the debug web viewer.
    Serve {
        /// Address to listen on; 0.0.0.0 to reach it from other machines.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8058)]
        port: u16,
        /// Directory of .fec files to list as samples.
        #[arg(long, default_value = DEFAULT_SAMPLES)]
        samples: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Print { filing, full, out } => {
            let receipt = render(&filing, full)?;
            warn(&receipt);
            let bytes = escpos::encode(&receipt);
            match out {
                Some(path) => {
                    std::fs::write(&path, &bytes)?;
                    eprintln!("wrote {} bytes to {}", bytes.len(), path.display());
                }
                None => {
                    usb::print(&bytes)?;
                    eprintln!("printed (~{:.0} mm of paper)", preview::length_mm(&receipt));
                }
            }
        }
        Command::Preview { filing, full, html } => {
            let receipt = render(&filing, full)?;
            match html {
                Some(path) => {
                    let page = format!(
                        "<!doctype html><meta charset=utf-8><title>Receipt Preview</title><style>body{{background:#e7e5e0;display:flex;justify-content:center;padding:32px 16px}}{}</style>{}",
                        preview::PAPER_CSS,
                        preview::html(&receipt)
                    );
                    std::fs::write(&path, page)?;
                    warn(&receipt);
                    eprintln!("wrote {}", path.display());
                }
                None => {
                    print!(
                        "{}",
                        preview::text(&receipt, std::io::stdout().is_terminal())
                    );
                    eprintln!("~{:.0} mm of paper", preview::length_mm(&receipt));
                }
            }
        }
        Command::Serve {
            host,
            port,
            samples,
        } => {
            serve::run(&host, port, samples.is_dir().then_some(samples))?;
        }
    }
    Ok(())
}

fn render(input: &str, full: bool) -> anyhow::Result<receipt::Receipt> {
    let mut filing = source::open(input)?;
    Ok(forms::render(&mut filing, forms::Opts { full }))
}

fn warn(receipt: &receipt::Receipt) {
    for w in &receipt.warnings {
        eprintln!("warning: {w}");
    }
}
