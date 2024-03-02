use std::{
    iter::repeat,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use color_eyre::eyre::Context;
use rand::seq::SliceRandom;
use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::DaemonArgs;

fn is_image(f: &PathBuf) -> bool {
    f.extension().map_or(false, |ext| {
        ["gif", "png", "jpg", "jpeg"]
            .into_iter()
            .any(|img_ext| img_ext == ext)
    })
}

fn get_wallpapers(wallpaper_dir: PathBuf) -> Vec<PathBuf> {
    let mut wallpapers: Vec<_> = std::fs::read_dir(&wallpaper_dir)
        .wrap_err_with(|| format!("wallpaper directory not found: {:?}", wallpaper_dir))
        .unwrap()
        .filter_map(|result| result.map(|entry| entry.path()).ok())
        .filter(is_image)
        .collect();
    wallpapers.shuffle(&mut rand::thread_rng());
    wallpapers
}

fn change_wallpaper(image: PathBuf) {
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
        .unwrap();
}

#[derive(Clone, Debug)]
enum PomoState {
    Work,
    ShortBreak,
    LongBreak,
}

impl std::fmt::Display for PomoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PomoState::Work => "work",
                PomoState::ShortBreak => "short break",
                PomoState::LongBreak => "long break",
            }
        )
    }
}

struct Job {
    filepath: PathBuf,
    sleep_dur: Duration,
    pomo_state: Option<PomoState>,
}

const WORK_SECS: u64 = 20;
const SHORT_BREAK_SECS: u64 = 5;
const LONG_BREAK_SECS: u64 = 15;
const REGULAR_SECS: u64 = 6;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum WorkerMessage {
    TogglePomo,
    Time,
    Skip,
}

pub struct Worker {
    pomo_state: Option<PomoState>,
    start_time: Instant,
    sleep_dur: Duration,
    job_iterator: Box<dyn Iterator<Item = Job>>,
    remain_in_job: bool,
    rx: Receiver<WorkerMessage>,
    args: DaemonArgs,
}

impl Worker {
    pub fn new(rx: Receiver<WorkerMessage>, args: DaemonArgs) -> Self {
        Self {
            pomo_state: None,
            // ugly but this will be overwritten in the first loop
            sleep_dur: Duration::from_secs(0),
            start_time: Instant::now(),
            job_iterator: Box::new(generate_jobs(false, &args.wallpaper_dir)),
            remain_in_job: true,
            rx,
            args,
        }
    }

    fn remaining(&self) -> Duration {
        self.sleep_dur - self.start_time.elapsed()
    }

    fn handle_message(&mut self, msg: WorkerMessage) {
        match msg {
            WorkerMessage::TogglePomo => {
                // pomo state will be overwritten in the next iteration of the run loop,
                // so there's no need to update it here
                self.job_iterator =
                    generate_jobs(self.pomo_state.is_none(), &self.args.wallpaper_dir);
                self.remain_in_job = false;
            }
            WorkerMessage::Time => {
                let remaining_str = format_duration(self.remaining());
                if let Some(pomo) = self.pomo_state.as_ref() {
                    notify(&format!("{remaining_str} remaining in {pomo}",))
                } else {
                    notify(&format!("{remaining_str} remaining on current wallpaper"))
                }
            }
            WorkerMessage::Skip => self.remain_in_job = false,
        }
    }

    pub async fn run(mut self) {
        loop {
            let current_job = self.job_iterator.next().unwrap();
            (self.pomo_state, self.sleep_dur) = (current_job.pomo_state, current_job.sleep_dur);
            if let Some(pomo) = self.pomo_state.as_ref() {
                notify(&format!("entering {pomo}"))
            }

            change_wallpaper(current_job.filepath);

            self.remain_in_job = true;
            self.start_time = Instant::now();
            while self.remain_in_job {
                // need this as self.rx.recv() borrows as mutable
                let remaining = self.remaining();
                tokio::select! {
                    Some(msg) = self.rx.recv() => self.handle_message(msg),
                    _ = sleep(remaining) => self.remain_in_job = false,
                }
            }
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    format!("{}m{}s", secs / 60, secs % 60)
}

fn notify(body: &str) {
    libnotify::Notification::new("cadence", Some(body), None)
        .show()
        .unwrap()
}

// this needs to be outside of worker as it's used in the constructor
fn generate_jobs(pomo: bool, wallpaper_dir: &PathBuf) -> Box<dyn Iterator<Item = Job>> {
    let images = get_wallpapers(wallpaper_dir.clone());
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
            pomo_state: new_pomo_state,
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
