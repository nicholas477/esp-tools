use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use log::{error, info};
use zip::{CompressionMethod, write::FileOptions};

use crate::{args, assets};

pub struct EspFileGraph {
    pub plugin: assets::AssetRef,
    pub graph: assets::AssetGraph,
    pub plugin_path: PathBuf,
}

pub async fn scan_esp_file_graph(
    input_file: &Path,
) -> Result<EspFileGraph, Box<dyn std::error::Error>> {
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

    let plugin_path = input_file.parent().unwrap();
    let plugin_asset = assets::Asset {
        kind: assets::Type::Plugin,
        path: assets::AssetPath {
            relative_path: input_file.strip_prefix(plugin_path)?.to_path_buf(),
        },
    };

    let (graph, plugin) = assets::AssetGraph::new_with_root(&plugin_asset);

    // Collect all asset references and their dependencies for the plugin in the ESP file.
    let scanned_assets = std::sync::Arc::new(dashmap::DashSet::new());
    assets::collect_references(&graph, &plugin, &plugin_path, &scanned_assets).await;

    Ok(EspFileGraph {
        plugin: plugin,
        graph: graph,
        plugin_path: plugin_path.to_path_buf(),
    })
}

pub fn get_esp_file_assets(plugin: &assets::AssetRef) -> HashSet<assets::AssetRef> {
    let mut assets = HashSet::new();
    assets.insert(plugin.clone());
    for child in plugin.children(false) {
        if let Some(asset) = child.upgrade() {
            if asset.asset.kind != assets::Type::Plugin {
                assets.insert(child.clone());
                assets.extend(child.children(true));
            }
        }
    }
    assets
}

// Removes master file assets from the provided set of assets and returns a new set containing only the master file assets that were removed.
pub fn remove_master_file_assets(
    assets: &mut HashSet<assets::AssetRef>,
) -> HashSet<assets::AssetRef> {
    let mut master_file_assets = HashSet::new();

    assets.retain(|asset| {
        if asset.upgrade().unwrap().kind == assets::Type::Plugin {
            master_file_assets.insert(asset.clone());
            info!(
                "Excluding master file {} from package",
                asset.upgrade().unwrap().path.relative_path.display()
            );
            return false;
        }
        true
    });

    master_file_assets
}

#[allow(dead_code)]
fn print_children(asset: &assets::AssetRef, depth: usize) {
    let indent = "  ".repeat(depth);
    info!(
        "{}- {} ({:?})",
        indent,
        asset.upgrade().unwrap().path.relative_path.display(),
        asset.upgrade().unwrap().kind
    );
    for child in asset.children(false) {
        print_children(&child, depth + 1);
    }
}

pub async fn package_esp_file(
    args: &args::PackageCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_file = &args.file;

    let EspFileGraph {
        plugin,
        graph,
        plugin_path,
    } = scan_esp_file_graph(input_file).await?;

    let mut assets = get_esp_file_assets(&plugin);
    let _removed_assets = remove_master_file_assets(&mut assets);

    info!("-- Zipping {} files:", assets.len());
    for asset in &assets {
        info!("\t-- {asset}");
    }

    // output path
    let zip_path = args
        .clone()
        .output
        .unwrap_or(Path::new(&input_file).with_extension("zip"));
    info!("Creating zip file at: \"{}\"", zip_path.display());

    zip_files(
        &assets
            .iter()
            .map(|asset| asset.upgrade().unwrap().path.relative_path.clone())
            .collect::<Vec<PathBuf>>(),
        &plugin_path,
        &zip_path,
    )?;

    info!(
        "Zip file created successfully at: \"{}\"",
        zip_path.display()
    );

    Ok(())
}

pub fn zip_files(
    files: &[PathBuf],
    data_files_path: &Path,
    output_zip_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(output_zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::DEFLATE);

    for file_path in files {
        let path = data_files_path.join(file_path);
        if path.is_file() {
            let zip_path = file_path.to_str().ok_or_else(|| {
                format!("Asset path is not valid Unicode: {}", file_path.display())
            })?;
            zip.start_file(zip_path, options)?;
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
