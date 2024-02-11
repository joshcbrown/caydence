use color_eyre::eyre::{Context, OptionExt, Result};
use rand::seq::IteratorRandom;
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    process::Command,
    time::Duration,
};
use tokio::sync::mpsc::{error::TryRecvError, Sender};
use tokio::{
    sync::mpsc::{self, Receiver},
    time::interval,
};

mod worker;

fn is_image(f: &PathBuf) -> bool {
    f.extension().map_or(false, |ext| {
        [".gif", ".png", ".jpg", ".jpeg"]
            .into_iter()
            .any(|img_ext| img_ext == ext)
    })
}

fn change_wallpaper() -> Result<()> {
    let wallpaper_dir = "/home/josh/.config/sway/wallpapers/";
    let files = std::fs::read_dir(wallpaper_dir)
        .wrap_err_with(|| format!("wallpaper directory not found: {wallpaper_dir}"))?;
    let foo = files
        .map(|result| result.expect("file changed during read").path())
        .filter(is_image);
    Command::new("swww")
        .args([
            "img",
            foo.choose(&mut rand::thread_rng())
                .ok_or_eyre("no files left")?
                .to_str()
                .expect("file was just found"),
            "--transition-fps",
            "140",
            "--transition-type",
            "center",
        ])
        .output()
        .wrap_err("swww failed to terminate")
        .map(|_| ())
}

// was high while writing this so i think it's dog shit
type ShouldClose = bool;

async fn handle_connection(mut stream: UnixStream, tx: Sender<Terminate>) -> ShouldClose {
    let mut message = String::new();
    stream.read_to_string(&mut message).unwrap();
    match message.as_str() {
        "change" => change_wallpaper().unwrap(),
        "toggle" => tx.send(Terminate).await.unwrap(),
        _ => {}
    }
    write!(stream, "success").unwrap();
    message == "close"
}

struct Terminate;

async fn loop_wallpapers(mut rx: Receiver<Terminate>) {
    let mut interval = interval(Duration::from_secs(5));
    interval.tick().await;
    while let Err(TryRecvError::Empty) = rx.try_recv() {
        change_wallpaper().unwrap();
        interval.tick().await;
    }
}

async fn setup_sockets() -> Result<()> {
    // TODO: move this elsewhere
    let (tx, rx) = mpsc::channel::<Terminate>(10);
    tokio::spawn(loop_wallpapers(rx));
    println!("over here");

    let listener = UnixListener::bind("/tmp/rallpaper.sock")
        .context("couldn't establish message handler connection")?;
    for result in listener.incoming() {
        let stream = result.wrap_err("socket borked")?;
        if handle_connection(stream, tx.clone()).await {
            // close call received
            break;
        }
    }
    std::fs::remove_file("/tmp/rallpaper.sock")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_sockets().await?;
    Ok(())
}
