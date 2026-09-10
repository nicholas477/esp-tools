use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// Opens the GUI for the command
    #[cfg(target_os = "windows")]
    #[arg(long)]
    pub gui: bool,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about = "Grabs the statics from an ESP file, then packages the meshes, textures, and the ESP file into a single zip.", long_about = None)]
pub struct PackageCommand {
    /// ESP file to isolate meshes and textures from
    pub file: PathBuf,

    /// (Optional) Output file path. If not specified, the zip file will be created in the same directory as the input ESP file.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Include assets that are also referenced by master plugins.
    /// If specified, the program will include assets that are referenced by master files into the zip file.
    #[arg(
        short,
        long,
        help = "Include assets that are also referenced by master plugins.\
        \nIf specified, the program will include assets that are referenced by master files into the zip file.\
        \nBy default, assets that are referenced by master files will be excluded from the zip file."
    )]
    pub include_master_files: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Package an ESP file into a zip
    Package(PackageCommand),

    /// Update the program to the latest version.
    Update,
}
