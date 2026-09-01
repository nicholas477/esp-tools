use std::{collections::HashSet, path::Path};

use log::{error, info};
use zip::{CompressionMethod, write::FileOptions};

use crate::{args, assets};

pub async fn package_esp_file(
    args: &args::PackageCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = &args.file;

    if !input_file.is_file() {
        return Err(format!(
            "The provided path is not a file: \"{}\"",
            input_file.display()
        )
        .into());
    }

    if !input_file.exists() {
        return Err(format!(
            "The provided file does not exist: \"{}\"",
            input_file.display()
        )
        .into());
    }

    // output path
    let zip_path = args
        .clone()
        .output
        .unwrap_or(Path::new(&input_file).with_extension("zip"));

    let plugin_path = input_file.parent().unwrap();
    let plugin = assets::load_plugin(&input_file).await?;

    // Collect all mesh references and their textures for the statics in the ESP file.
    let mut files = assets::collect_asset_files(&plugin, &input_file, plugin_path, true).await?;
    files.insert(
        input_file
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .ok_or("Input file name is not valid Unicode")?
            .to_string(),
    );

    if !args.include_master_files {
        // Lookup master plugin files and remove any assets that are also referenced by them.
        let master_plugin_files =
            assets::collect_master_plugin_files(&plugin, &input_file, plugin_path).await?;

        {
            info!(
                "-- Excluding assets from {} master plugins:",
                master_plugin_files.len()
            );
            let mut master_plugin_files = master_plugin_files.iter().collect::<Vec<_>>();
            master_plugin_files.sort();
            for master_plugin_file in master_plugin_files {
                info!(
                    "\t-- {}",
                    master_plugin_file
                        .file_name()
                        .unwrap_or(master_plugin_file.as_os_str())
                        .to_string_lossy()
                );
            }
        }

        assets::remove_master_asset_files(&mut files, &master_plugin_files, plugin_path).await?;
    }

    {
        info!("-- Zipping {} files:", files.len());
        let mut files = files.iter().collect::<Vec<&String>>();
        files.sort();
        for file in &files {
            info!("\t-- {file}");
        }
    }

    info!("Creating zip file at: \"{}\"", zip_path.display());

    zip_files(&files, plugin_path, &zip_path)?;

    info!(
        "Zip file created successfully at: \"{}\"",
        zip_path.display()
    );

    Ok(())
}

fn zip_files(
    files: &HashSet<String>,
    data_files_path: &Path,
    output_zip_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(output_zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::DEFLATE);

    for file_path in files {
        let path = data_files_path.join(Path::new(file_path));
        if path.is_file() {
            zip.start_file(file_path, options)?;
            let mut f = std::fs::File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        } else {
            error!("Warning: File not found: {}", path.display());
            return Err(format!("File not found: {}", path.display()).into());
        }
    }

    // Redundant since the zip is finished when the ZipWriter is dropped
    zip.finish()?;

    Ok(())
}
