use std::{net::Ipv4Addr, ops::Range};

use anyhow::{bail, Result};
use structopt::StructOpt;

fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(opts.target, &opts.ports, &opts.ranges) {
        eprintln!("tsunami: {:?}", e);
    }
}

fn run(target: Ipv4Addr, ports: &[u16], ranges: &[PortRange]) -> Result<()> {
    Ok(())
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    target: Ipv4Addr,

    #[structopt(short, long)]
    ports: Vec<u16>,

    #[structopt(short, long)]
    ranges: Vec<PortRange>,
}

#[derive(Debug, Clone, Copy)]
struct PortRange {
    start: u16,
    end: u16,
}

impl From<Vec<u16>> for PortRange {
    fn from(value: Vec<u16>) -> Self {
        Self {
            start: value[0],
            end: value[1],
        }
    }
}

impl std::str::FromStr for PortRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split("-").collect();
        if parts.len() != 2 {
            bail!("expected start-end");
        }

        let parsed: Vec<u16> = parts
            .iter()
            .map(|p| p.parse::<u16>().expect("can't parse u16"))
            .collect();

        Ok(parsed.into())
    }
}
