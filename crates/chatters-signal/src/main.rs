use chatters_lib::log::init_logger;
use chatters_lib::util::{self, Options};
use chatters_signal::Signal;
use clap::Parser;
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(name = "chatters-signal")]
pub struct Arguments {
    #[clap(long, default_value = "chatters-signal")]
    device_name: String,

    #[clap(long)]
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let project_dirs = ProjectDirs::from("net", "jeffas", "chatters-signal").unwrap();
    let data_local_dir = project_dirs.data_local_dir();

    let log_path = data_local_dir.join("logs.log");
    init_logger(log_path);

    let args = Arguments::parse();

    let config_file = match args.config_file {
        Some(cf) => cf,
        None => project_dirs.config_local_dir().join("config.toml"),
    };

    let options = Options {
        device_name: args.device_name,
        data_local_dir: data_local_dir.to_owned(),
        config_file,
        app_name: "chatters-signal".to_owned(),
    };

    util::run::<Signal>(options).await;

    Ok(())
}
