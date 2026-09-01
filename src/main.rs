#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use ::log::error;
use clap::Parser;

mod args;
mod assets;
mod commands;
mod log;
mod update;

#[cfg(target_os = "windows")]
mod windows;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn main() {
    log::init_logger();
    let args = args::Args::parse();

    let exit_code = match run(&args) {
        Ok(()) => 0,
        Err(error) => {
            error!("{error}");
            1
        }
    };

    std::process::exit(exit_code);
}

fn run(args: &args::Args) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    if args.gui {
        return windows::run(args);
    }

    runtime().block_on(async {
        match &args.command {
            args::Commands::Package(package_command) => {
                commands::package::package_esp_file(package_command).await?;
            }
            args::Commands::Update => {
                update::update().await?;
            }
        }

        Ok(())
    })
}
