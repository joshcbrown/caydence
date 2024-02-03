use color_eyre::eyre::{Context, OptionExt, Result};
use rand::seq::IteratorRandom;
use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    process::Command,
};

fn change_wallpaper() -> Result<()> {
    let wallpaper_dir = "/home/josh/.config/sway/wallpapers/";
    let files = std::fs::read_dir(wallpaper_dir)
        .wrap_err_with(|| format!("wallpaper directory not found: {wallpaper_dir}"))?;
    let foo = files.map(|dir_entry| dir_entry.unwrap().path());
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
        .wrap_err_with(|| "swww failed to terminate")
        .map(|_| ())
}

type ShouldClose = bool;

fn handle_connection(mut stream: UnixStream) -> ShouldClose {
    let mut message = String::new();
    stream.read_to_string(&mut message).unwrap();
    println!("message received: {message}!");
    write!(stream, "success").unwrap();
    message == "close"
}

fn main() -> Result<()> {
    // random ass port that hopefully doesn't clash with anything else
    let listener = UnixListener::bind("/tmp/rallpaper.sock")
        .context("couldn't establish message handler connection")?;
    let _ = listener
        .incoming()
        .map(|result| result.map(|stream| handle_connection(stream)))
        // incoming() is an infinite iterator, so we need to use try_for_each to return early
        .try_for_each(|result| {
            if let Ok(true) = result {
                Err("close call received")
            } else {
                Ok(())
            }
        });
    std::fs::remove_file("/tmp/rallpaper.sock")?;
    Ok(())
}
