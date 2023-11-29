use crate::Port;
use anyhow::{bail, Result};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(short, long)]
    pub target: String,

    #[structopt(short, long)]
    pub ports: Vec<Port>,

    #[structopt(short, long)]
    pub ranges: Vec<PortRange>,

    #[structopt(short, long, default_value = "512")]
    pub flying_tasks: u16,

    #[structopt(short, long, default_value = "3")]
    pub max_retries: usize,

    #[structopt(short, long, default_value = "512")]
    pub batch_size: usize,

    #[structopt(short = "n", long, default_value = "10")]
    pub nap_after_spawn: f64,

    #[structopt(short = "N", long, default_value = "10")]
    pub nap_after_batch: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PortRange {
    pub start: Port,
    pub end: Port,
}

impl From<Vec<Port>> for PortRange {
    fn from(value: Vec<Port>) -> Self {
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

        let parsed: Vec<Port> = parts
            .iter()
            .map(|p| p.parse::<Port>().expect("can't parse port as u16"))
            .collect();

        Ok(parsed.into())
    }
}
