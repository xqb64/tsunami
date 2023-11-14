use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};
use structopt::StructOpt;
use tokio::sync::Semaphore;
use tsunami::cli::{Opt, PortRange};
use tsunami::receiver::receive;
use tsunami::worker::inspect;

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(opts.target, &opts.ports, &opts.ranges).await {
        eprintln!("tsunami: {:?}", e);
    }
}

async fn run(target: Ipv4Addr, ports: &[u16], ranges: &[PortRange]) -> Result<()> {
    let mut tasks = vec![];

    let receiver = tokio::spawn(receive());

    let id_table = Arc::new(Mutex::new(HashMap::new()));
    let semaphore = Arc::new(Semaphore::new(512));

    let combined: HashSet<_> = ports
        .iter()
        .copied()
        .chain(ranges.iter().flat_map(|r| (r.start..r.end)))
        .collect();

    for port in combined {
        tasks.push(tokio::spawn(inspect(
            target,
            port,
            semaphore.clone(),
            id_table.clone(),
        )));
    }

    for task in tasks {
        task.await??;
    }

    receiver.await??;

    Ok(())
}
