use chatters::backends::signal::Signal;
use chatters::log::init_logger;
use chatters::util;
use clap::Parser;
use directories::ProjectDirs;

#[derive(Debug, Parser)]
#[clap(name = "chatters-signal")]
pub struct Options {
    #[clap(long, default_value = "chatters-signal")]
    device_name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let project_dirs = ProjectDirs::from("net", "jeffas", "chatters-signal").unwrap();

    let log_path = project_dirs.data_local_dir().join("logs.log");
    init_logger(log_path);

    let opts = Options::parse();

    util::run::<Signal>(&opts.device_name, &project_dirs).await;

    Ok(())
}
