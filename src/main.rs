use clap::Parser;
use runsvdir::step;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::{io, thread};
use tracing::error;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The number of millis to wait between each check
    #[clap(short, long, default_value = "1000")]
    pause: u64,
    /// The directory to store process states
    dir: PathBuf,
}

fn main() {
    let args = Args::parse();

    FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_writer(io::stderr)
        .init();

    let mut running = HashMap::new();
    let pause = Duration::from_millis(args.pause);

    loop {
        if let Err(err) = step(&args.dir, &mut running) {
            error!("step failed: {err}");
        }
        thread::sleep(pause);
    }
}
