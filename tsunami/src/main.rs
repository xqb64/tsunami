use anyhow::Result;
use std::{collections::HashSet, sync::Arc};
use structopt::StructOpt;
use tokio::sync::{mpsc, Semaphore};
use tsunami::cli::{Opt, PortRange};
use tsunami::net::to_ipaddr;
use tsunami::receiver::receive;
use tsunami::worker::inspect;
use tsunami::Message;

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();
    if let Err(e) = run(&opts.target, &opts.ports, &opts.ranges, opts.workers).await {
        eprintln!("tsunami: {:?}", e);
    }
}

async fn run(target: &str, ports: &[u16], ranges: &[PortRange], workers: u16) -> Result<()> {
    let combined: HashSet<_> = ports
        .iter()
        .copied()
        .chain(ranges.iter().flat_map(|r| (r.start..r.end)))
        .collect();

    let (tx, mut rx) = mpsc::channel(8);

    let receiver = tokio::spawn(receive(combined, tx));

    let semaphore = Arc::new(Semaphore::new(workers as usize));

    loop {
        if let Some(msg) = rx.recv().await {
            match msg {
                Message::Payload(payload) => {
                    let mut tasks = vec![];

                    for port in payload {
                        tasks.push(tokio::spawn(inspect(
                            to_ipaddr(target).await?,
                            port,
                            semaphore.clone(),
                        )));
                    }

                    for task in tasks {
                        task.await??;
                    }
                }
                Message::Break => break,
            }
        }
    }

    receiver.await??;

    Ok(())
}
