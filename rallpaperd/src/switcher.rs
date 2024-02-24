use color_eyre::eyre::{Context, OptionExt, Result};
use rand::seq::IteratorRandom;
use std::{iter::repeat, path::PathBuf, process::Command, time::Duration};

use tokio::{
    sync::mpsc::{self, error::TryRecvError, Receiver, Sender},
    time::{interval, sleep},
};

#[derive(Debug, Clone)]
struct SwitchData {
    wallpaper_duration: Duration,
    wallpaper_path: PathBuf,
}

fn is_image(f: &PathBuf) -> bool {
    f.extension().map_or(false, |ext| {
        [".gif", ".png", ".jpg", ".jpeg"]
            .into_iter()
            .any(|img_ext| img_ext == ext)
    })
}

fn get_wallpapers(wallpaper_dir: PathBuf) -> Result<impl Iterator<Item = PathBuf>> {
    Ok(std::fs::read_dir(&wallpaper_dir)
        .wrap_err_with(|| format!("wallpaper directory not found: {:?}", wallpaper_dir))?
        .filter_map(|result| result.map(|entry| entry.path()).ok())
        .filter(is_image))
}

fn change_wallpaper(image: PathBuf) -> Result<()> {
    Command::new("swww")
        .args([
            "img",
            image.as_os_str().to_str().unwrap(),
            "--transition-fps",
            "140",
            "--transition-type",
            "center",
        ])
        .output()
        .wrap_err("swww failed to terminate")
        .map(|_| ())
}

fn rand_wallpaper_from(dir: PathBuf) -> Result<()> {
    let images = get_wallpapers(dir)?;
    let chosen = images
        .choose(&mut rand::thread_rng())
        .ok_or_eyre("no files left")?;
    change_wallpaper(chosen)
}

struct Terminate;

struct Switcher {
    tx: Sender<Terminate>,
}

impl Switcher {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Terminate>(1);
        tokio::spawn(Looper::new(rx).unwrap().loop_wallpapers());
        Self { tx }
    }

    pub async fn toggle(&mut self) {
        self.tx.send(Terminate).await.unwrap();
        *self = Self::new();
    }
}

struct Looper {
    rx: Receiver<Terminate>,
    in_pomo: bool,
}

const WORK_SECS: u64 = 5;
const SHORT_BREAK_SECS: u64 = 2;
const LONG_BREAK_SECS: u64 = 3;
const REGULAR_SECS: u64 = 6;

impl Looper {
    fn new(rx: Receiver<Terminate>) -> Result<Self> {
        Ok(Self { rx, in_pomo: false })
    }

    fn new_queue(&self) -> Result<impl Iterator<Item = (Duration, PathBuf)>> {
        // need to collect to satisfy clone trait down ▼
        let images: Vec<_> = get_wallpapers("/home/josh/.config/sway/wallpapers".into())?.collect();
        let times: Box<dyn Iterator<Item = Duration>> = if self.in_pomo {
            Box::new(
                [
                    Duration::from_secs(WORK_SECS),
                    Duration::from_secs(SHORT_BREAK_SECS),
                    Duration::from_secs(LONG_BREAK_SECS),
                ]
                .into_iter()
                .cycle(),
            )
        } else {
            Box::new(repeat(Duration::from_secs(REGULAR_SECS)))
        };
        // here on the inside of the zip call
        Ok(times.zip(images.into_iter().cycle()))
    }

    pub async fn loop_wallpapers(mut self) {
        let mut queue = self.new_queue().unwrap();
        while let Err(TryRecvError::Empty) = self.rx.try_recv() {
            let (dur, path) = queue.next().unwrap();
            change_wallpaper(path).unwrap();
            sleep(dur.clone()).await;
        }
    }
}
