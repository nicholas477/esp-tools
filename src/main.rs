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

#[tokio::main]
async fn main() {
    log::init_logger();
    let args = args::Args::parse();

    let exit_code = match run(args.clone()).await {
        Ok(()) => 0,
        Err(error) => {
            error!("{error}");
            1
        }
    };

    std::process::exit(exit_code);
}

async fn run(args: args::Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        args::Commands::Package(package_command) => {
            #[cfg(target_os = "windows")]
            if args.gui {
                if windows::run(&package_command.file)? == windows::GuiAction::Package {
                    commands::package::package_esp_file(package_command.clone()).await?;
                }
            } else {
                commands::package::package_esp_file(package_command.clone()).await?;
            }

            #[cfg(not(target_os = "windows"))]
            commands::package::package_esp_file(package_command.clone()).await?;
        }
        args::Commands::Update => {
            update::update()?;
        }
    }

    Ok(())
}
