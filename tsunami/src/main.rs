use anyhow::Result;
use std::{collections::HashSet, sync::Arc};
use structopt::StructOpt;
use tokio::sync::Semaphore;
use tsunami::cli::{Opt, PortRange};
use tsunami::net::to_ipaddr;
use tsunami::receiver::receive;
use tsunami::worker::inspect;

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(&opts.target, &opts.ports, &opts.ranges, opts.workers).await {
        eprintln!("tsunami: {:?}", e);
    }
}

async fn run(target: &str, ports: &[u16], ranges: &[PortRange], workers: u16) -> Result<()> {
    let mut tasks = vec![];

    let receiver = tokio::spawn(receive());

    let semaphore = Arc::new(Semaphore::new(workers as usize));

    let combined: HashSet<_> = ports
        .iter()
        .copied()
        .chain(ranges.iter().flat_map(|r| (r.start..r.end)))
        .collect();

    for port in combined {
        tasks.push(tokio::spawn(inspect(
            to_ipaddr(target).await?,
            port,
            semaphore.clone(),
        )));
    }

    for task in tasks {
        task.await??;
    }

    receiver.await??;

    Ok(())
}
