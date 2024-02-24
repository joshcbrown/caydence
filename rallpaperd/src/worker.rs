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
        ["gif", "png", "jpg", "jpeg"]
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

#[derive(Clone, Debug)]
enum PomoState {
    Work,
    ShortBreak,
    LongBreak,
}

impl PomoState {
    pub fn dur(&self) -> Duration {
        match self {
            PomoState::Work => Duration::from_secs(WORK_SECS),
            PomoState::ShortBreak => Duration::from_secs(SHORT_BREAK_SECS),
            PomoState::LongBreak => Duration::from_secs(LONG_BREAK_SECS),
        }
    }
}

struct Job {
    filepath: PathBuf,
    sleep_dur: Duration,
    new_pomo_state: Option<PomoState>,
}

const WORK_SECS: u64 = 5;
const SHORT_BREAK_SECS: u64 = 2;
const LONG_BREAK_SECS: u64 = 3;
const REGULAR_SECS: u64 = 6;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum WorkerMessage {
    TogglePomo,
    Time,
    ChangeWallpaper,
}

pub struct Worker {
    pomo_state: Option<PomoState>,
    start_time: Instant,
    job_iterator: Box<dyn Iterator<Item = Job>>,
    rx: Receiver<WorkerMessage>,
}

impl Worker {
    pub fn new(rx: Receiver<WorkerMessage>) -> Self {
        Self {
            pomo_state: None,
            start_time: Instant::now(),
            job_iterator: Box::new(generate_jobs(false)),
            rx,
        }
    }

    fn handle_message(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::TogglePomo => {
                // pomo state will be overwritten in the next iteration of the run loop,
                // so there's no need to update it here
                self.job_iterator = generate_jobs(self.pomo_state.is_none());
            }
            WorkerMessage::Time => {
                if let Some(pomo) = self.pomo_state.as_ref() {
                    notify(&format!(
                        "{:?}-{:?} remaining",
                        pomo,
                        pomo.dur() - self.start_time.elapsed()
                    ))
                } else {
                    notify("not in pomo")
                }
            }
            WorkerMessage::ChangeWallpaper => {}
        }
    }

    pub async fn run(mut self) {
        loop {
            let current_job = self.job_iterator.next().unwrap();
            self.pomo_state = current_job.new_pomo_state;
            self.start_time = Instant::now();
            if let Some(pomo) = self.pomo_state.as_ref() {
                notify(&format!("{:?}", pomo))
            }

            change_wallpaper(current_job.filepath).unwrap();

            tokio::select! {
                Some(msg) = self.rx.recv() => self.handle_message(msg),
                _ = sleep(current_job.sleep_dur) => { }
            }
        }
    }
}

fn notify(body: &str) {
    libnotify::Notification::new("cadence", Some(body), None)
        .show()
        .unwrap()
}

// this needs to be outside of worker as it's used in the constructor
fn generate_jobs(pomo: bool) -> Box<dyn Iterator<Item = Job>> {
    let images: Vec<_> = get_wallpapers("/home/josh/.config/sway/wallpapers".into()).collect();
    let times: Box<dyn Iterator<Item = (Duration, Option<PomoState>)>> = if pomo {
        Box::new(
            [
                (Duration::from_secs(WORK_SECS), Some(PomoState::Work)),
                (
                    Duration::from_secs(SHORT_BREAK_SECS),
                    Some(PomoState::ShortBreak),
                ),
                (Duration::from_secs(WORK_SECS), Some(PomoState::Work)),
                (
                    Duration::from_secs(SHORT_BREAK_SECS),
                    Some(PomoState::ShortBreak),
                ),
                (Duration::from_secs(WORK_SECS), Some(PomoState::Work)),
                (
                    Duration::from_secs(SHORT_BREAK_SECS),
                    Some(PomoState::ShortBreak),
                ),
                (Duration::from_secs(WORK_SECS), Some(PomoState::Work)),
                (
                    Duration::from_secs(LONG_BREAK_SECS),
                    Some(PomoState::LongBreak),
                ),
            ]
            .into_iter()
            .cycle(),
        )
    } else {
        Box::new(repeat((Duration::from_secs(REGULAR_SECS), None)))
    };
    Box::new(times.zip(images.into_iter().cycle()).map(
        |((sleep_dur, new_pomo_state), filepath)| Job {
            sleep_dur,
            filepath,
            new_pomo_state,
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn image_extension_works() {
        let ex = PathBuf::from_str("example.png").unwrap();
        assert!(is_image(&ex))
    }
}
