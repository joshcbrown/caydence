use color_eyre::eyre::Result;
use listener::Listener;
use tokio::sync::mpsc;
use worker::Worker;

mod listener;
mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    libnotify::init("cadence").unwrap();
    let (tx_worker, rx_worker) = mpsc::channel(10);

    tokio::spawn(Listener::new(tx_worker).unwrap().run());

    Worker::new(rx_worker).run().await;

    Ok(())
}
