use std::{
    iter::repeat,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use color_eyre::eyre::Context;
use rand::seq::SliceRandom;
use tokio::{sync::mpsc::Receiver, time::sleep};

use crate::{ClientCommand, DaemonArgs};

fn is_image(f: &Path) -> bool {
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
        .filter(|pb| is_image(pb))
        .collect();
    wallpapers.shuffle(&mut rand::thread_rng());
    wallpapers
}

fn change_wallpaper(image: PathBuf, transition_fps: u8) {
    Command::new("swww")
        .args([
            "img",
            image.as_os_str().to_str().unwrap(),
            "--transition-fps",
            &transition_fps.to_string(),
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

pub struct Worker {
    pomo_state: Option<PomoState>,
    start_time: Instant,
    paused: Option<TimeRemaining>,
    sleep_dur: Duration,
    job_iterator: Box<dyn Iterator<Item = Job>>,
    remain_in_job: bool,
    rx: Receiver<ClientCommand>,
    args: DaemonArgs,
}

struct TimeRemaining(Duration);

impl Worker {
    pub fn new(rx: Receiver<ClientCommand>, args: DaemonArgs) -> Self {
        Self {
            pomo_state: None,
            // ugly but this will be overwritten in the first loop
            sleep_dur: Duration::from_secs(0),
            start_time: Instant::now(),
            job_iterator: Box::new(generate_jobs(false, &args)),
            remain_in_job: true,
            paused: None,
            rx,
            args,
        }
    }

    fn remaining(&self) -> Duration {
        if let Some(TimeRemaining(t)) = self.paused {
            t
        } else {
            self.sleep_dur - self.start_time.elapsed()
        }
    }

    fn skip(&mut self) {
        self.remain_in_job = false;
        self.paused = None;
    }

    fn handle_message(&mut self, msg: ClientCommand) {
        match msg {
            ClientCommand::Pause => {
                if let Some(TimeRemaining(t)) = self.paused {
                    self.start_time = Instant::now() - self.sleep_dur + t;
                    self.paused = None;
                    notify("resuming");
                } else {
                    self.paused = Some(TimeRemaining(self.remaining()));
                    notify("pausing")
                }
            }
            ClientCommand::Toggle => {
                // pomo state will be overwritten in the next iteration of the run loop,
                // so there's no need to update it here
                self.job_iterator = generate_jobs(self.pomo_state.is_none(), &self.args);
                self.skip();
                if self.pomo_state.is_some() {
                    notify("exiting pomo");
                }
            }
            ClientCommand::Time => {
                let remaining_str = format_duration(self.remaining());
                if let Some(pomo) = self.pomo_state.as_ref() {
                    notify(&format!("{remaining_str} remaining in {pomo}",))
                } else {
                    notify(&format!("{remaining_str} remaining on current wallpaper"))
                }
            }
            ClientCommand::Skip => self.skip(),
        }
    }

    pub async fn run(mut self) {
        loop {
            let current_job = self.job_iterator.next().unwrap();
            (self.pomo_state, self.sleep_dur) = (current_job.pomo_state, current_job.sleep_dur);
            if let Some(pomo) = self.pomo_state.as_ref() {
                notify(&format!("entering {pomo}"))
            }

            change_wallpaper(current_job.filepath, self.args.transition_fps);
            self.remain_in_job = true;
            self.start_time = Instant::now();
            while self.remain_in_job {
                if self.paused.is_some() {
                    if let Some(msg) = self.rx.recv().await {
                        self.handle_message(msg)
                    }
                    continue;
                }
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
    libnotify::Notification::new("caydence", Some(body), None)
        .show()
        .unwrap()
}

fn dur_from_mins(mins: u8) -> Duration {
    Duration::from_secs(mins as u64 * 60)
}

// this needs to be outside of worker as it's used in the constructor
fn generate_jobs(pomo: bool, args: &DaemonArgs) -> Box<dyn Iterator<Item = Job>> {
    let images = get_wallpapers(args.wallpaper_dir.clone());
    let times: Box<dyn Iterator<Item = (Duration, Option<PomoState>)>> = if pomo {
        Box::new(
            [
                (dur_from_mins(args.work_mins), Some(PomoState::Work)),
                (
                    dur_from_mins(args.short_break_mins),
                    Some(PomoState::ShortBreak),
                ),
            ]
            .into_iter()
            .cycle()
            .take((args.cycles_before_break as usize - 1) * 2)
            .chain([
                (dur_from_mins(args.work_mins), Some(PomoState::Work)),
                (
                    dur_from_mins(args.long_break_mins),
                    Some(PomoState::LongBreak),
                ),
            ])
            .cycle(),
        )
    } else {
        Box::new(repeat((
            Duration::from_secs(args.regular_interval_mins as u64 * 60),
            None,
        )))
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
