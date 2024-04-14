use std::io::Write;
use std::{
    io::Read,
    os::unix::net::{UnixListener, UnixStream},
};

use color_eyre::eyre::{Context, Result};
use tokio::sync::mpsc::Sender;

use crate::ClientCommand;

pub struct Listener {
    tx_worker: Sender<ClientCommand>,
    listener: UnixListener,
}

async fn handle_connection(mut stream: UnixStream, tx: Sender<ClientCommand>) {
    let mut message = String::new();
    stream.read_to_string(&mut message).unwrap();
    let worker_message = match message.as_str() {
        "skip" => Some(ClientCommand::Skip),
        "toggle" => Some(ClientCommand::Toggle),
        "time" => Some(ClientCommand::Time),
        "pause" => Some(ClientCommand::Pause),
        _ => None,
    };
    if let Some(msg) = worker_message {
        tx.send(msg).await.unwrap();
        write!(stream, "success").unwrap();
    } else {
        write!(stream, "invalid message: {message}").unwrap();
    }
    stream.shutdown(std::net::Shutdown::Write).unwrap();
}

impl Listener {
    pub fn new(tx_worker: Sender<ClientCommand>) -> Result<Self> {
        let listener = UnixListener::bind("/tmp/rallpaper.sock")
            .context("couldn't establish message handler connection")?;
        Ok(Self {
            tx_worker,
            listener,
        })
    }

    pub async fn run(self) {
        for result in self.listener.incoming() {
            let stream = result.unwrap();
            handle_connection(stream, self.tx_worker.clone()).await;
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        std::fs::remove_file("/tmp/rallpaper.sock")
            .unwrap_or_else(|_| println!("problem destructing socket file"));
    }
}
