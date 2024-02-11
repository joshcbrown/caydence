use std::{path::PathBuf, time::Duration};

use color_eyre::eyre::Result;

struct Job {
    filepath: PathBuf,
    sleep_dur: Duration,
}

impl Job {
    async fn run(self) -> Result<()> {}
}
