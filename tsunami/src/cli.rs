use anyhow::{bail, Result};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(short, long)]
    pub target: String,

    #[structopt(short, long)]
    pub ports: Vec<u16>,

    #[structopt(short, long)]
    pub ranges: Vec<PortRange>,

    #[structopt(short, long, default_value = "1")]
    pub workers: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
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
        let parts: Vec<_> = s.split('-').collect();
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
