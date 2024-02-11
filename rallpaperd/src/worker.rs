use std::{
    iter::repeat,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use color_eyre::eyre::{Context, Result};
use tokio::{sync::mpsc::Receiver, time::sleep};

fn is_image(f: &PathBuf) -> bool {
    f.extension().map_or(false, |ext| {
        [".gif", ".png", ".jpg", ".jpeg"]
            .into_iter()
            .any(|img_ext| img_ext == ext)
    })
}

fn get_wallpapers(wallpaper_dir: PathBuf) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(&wallpaper_dir)
        .wrap_err_with(|| format!("wallpaper directory not found: {:?}", wallpaper_dir))
        .unwrap()
        .filter_map(|result| result.map(|entry| entry.path()).ok())
        .filter(is_image)
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

pub struct Job {
    pub filepath: PathBuf,
    pub sleep_dur: Duration,
}

const WORK_SECS: u64 = 5;
const SHORT_BREAK_SECS: u64 = 2;
const LONG_BREAK_SECS: u64 = 3;
const REGULAR_SECS: u64 = 6;

fn generate_jobs(pomo: bool) -> Box<dyn Iterator<Item = Job>> {
    let images: Vec<_> = get_wallpapers("/home/josh/.config/sway/wallpapers".into()).collect();
    let times: Box<dyn Iterator<Item = Duration>> = if pomo {
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
    Box::new(
        times
            .zip(images.into_iter().cycle())
            .map(|(sleep_dur, filepath)| Job {
                sleep_dur,
                filepath,
            }),
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum WorkerMessage {
    TogglePomo,
    Time,
    ChangeWallpaper,
}

pub struct Worker {
    in_pomo: bool,
    start_time: Instant,
    job_iterator: Box<dyn Iterator<Item = Job>>,
    rx: Receiver<WorkerMessage>,
}

impl Worker {
    pub fn new(rx: Receiver<WorkerMessage>) -> Self {
        Self {
            in_pomo: false,
            start_time: Instant::now(),
            job_iterator: Box::new(generate_jobs(false)),
            rx,
        }
    }

    fn handle_message(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::TogglePomo => {
                self.in_pomo = !self.in_pomo;
                self.job_iterator = generate_jobs(self.in_pomo);
                println!("switching pomo to {}", self.in_pomo)
            }
            WorkerMessage::Time => {
                dbg!(self.start_time.elapsed());
            }
            WorkerMessage::ChangeWallpaper => {}
        }
    }

    pub async fn run(mut self) {
        loop {
            let current_job = self.job_iterator.next().unwrap();
            self.start_time = Instant::now();
            println!("changing wallpaper!");
            change_wallpaper(current_job.filepath).unwrap();

            tokio::select! {
                Some(msg) = self.rx.recv() => self.handle_message(msg),
                _ = sleep(current_job.sleep_dur) => {
                    println!("waking up!");
                }
            }
        }
    }
}
