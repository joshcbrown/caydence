use color_eyre::eyre::Result;
use listener::Listener;
use tokio::sync::mpsc;
use worker::Worker;

mod listener;
mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx_worker, rx_worker) = mpsc::channel(1);

    // Spawn worker task
    tokio::spawn(async move {
        Worker::new(rx_worker).run().await;
    });

    Listener::new(tx_worker).unwrap().run();

    Ok(())
}
