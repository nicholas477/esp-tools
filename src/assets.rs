use ::log::{error, info};
use std::collections::hash_set::HashSet;
use std::path::{Path, PathBuf};
use tes3::esp::{Plugin, Static};
use tes3::nif::TextureSource::External;
use tes3::nif::{NiSourceTexture, NiStream};
use tokio::task::JoinSet;

mod types;
pub use types::*;

/// Verifies that a referenced file exists and reports the file that references it.
fn validate_referenced_file(
    source_file: &Path,
    referenced_path: &Path,
    full_path: &Path,
) -> Result<(), String> {
    if full_path.exists() {
        return Ok(());
    }

    error!(
        "Error: {} references nonexistent file: {}",
        source_file.display(),
        referenced_path.display()
    );
    error!("Looked for file at path: {}", full_path.display());
    Err(format!(
        "{} references nonexistent file: {}",
        source_file.display(),
        referenced_path.display()
    ))
}

/// Loads a mesh and returns its external textures, optionally validating its references.
fn scan_mesh_textures(
    mesh_path: &Path,
    plugin_file: &Path,
    plugin_path: &Path,
    validate_references: bool,
) -> Result<Vec<PathBuf>, String> {
    let mesh_asset_path = AssetPath {
        relative_path: mesh_path.to_path_buf(),
    };
    let full_path = mesh_asset_path.make_full(plugin_path);
    if validate_references {
        validate_referenced_file(plugin_file, mesh_path, &full_path)?;
    }

    let mut stream = NiStream::new();
    if let Err(error) = stream.load_path(&full_path) {
        if validate_references {
            return Err(format!(
                "Failed to load NIF {} referenced by {}: {error}",
                full_path.display(),
                plugin_file.display()
            ));
        }

        return Ok(Vec::new());
    }

    let mut textures = Vec::new();
    for object in stream.objects_of_type::<NiSourceTexture>() {
        if let External(file_name) = &object.source {
            let texture_path = Path::new(file_name);
            let texture_full_path = plugin_path.join(texture_path);

            if validate_references {
                validate_referenced_file(&full_path, texture_path, &texture_full_path)?;
            }
            textures.push(texture_path.to_path_buf());
        }
    }

    Ok(textures)
}

/// Loads a plugin in a blocking task so it does not block the async runtime.
async fn load_plugin_with_context(plugin_file: PathBuf, context: String) -> Result<Plugin, String> {
    let task_plugin_file = plugin_file.clone();
    tokio::task::spawn_blocking(move || {
        Plugin::from_path(&task_plugin_file).map_err(|error| format!("{context}: {error}"))
    })
    .await
    .map_err(|error| {
        format!(
            "Plugin load task failed for {}: {error}",
            plugin_file.display()
        )
    })?
}

/// Loads the input ESP or ESM in a blocking task.
pub async fn load_plugin(plugin_file: &Path) -> Result<Plugin, String> {
    let plugin_file = plugin_file.to_path_buf();
    let context = format!("Failed to load input plugin {}", plugin_file.display());
    load_plugin_with_context(plugin_file, context).await
}

/// Returns the canonical paths of every master ESP or ESM declared by a plugin.
pub async fn collect_master_plugin_files(
    plugin: &Plugin,
    plugin_file: &Path,
    plugin_path: &Path,
) -> Result<HashSet<PathBuf>, String> {
    let mut visited_plugins = HashSet::new();
    let mut pending_plugins = vec![(plugin.clone(), plugin_file.to_path_buf())];

    while let Some((plugin, plugin_file)) = pending_plugins.pop() {
        let Some(header) = plugin.header() else {
            continue;
        };

        for (master_name, _) in &header.masters {
            let master_path = plugin_path.join(master_name);
            let canonical_master_path = master_path.canonicalize().map_err(|error| {
                format!(
                    "Failed to resolve master plugin {} referenced by {}: {error}",
                    master_path.display(),
                    plugin_file.display()
                )
            })?;
            if !visited_plugins.insert(canonical_master_path.clone()) {
                continue;
            }

            let context = format!(
                "Failed to load master plugin {} referenced by {}",
                canonical_master_path.display(),
                plugin_file.display()
            );
            let master_plugin =
                load_plugin_with_context(canonical_master_path.clone(), context).await?;
            pending_plugins.push((master_plugin, canonical_master_path));
        }
    }

    Ok(visited_plugins)
}

/// Removes input assets that are also referenced by any discovered master plugin.
pub async fn remove_master_asset_files(
    assets: &AssetGraph,
    master_plugin_files: &HashSet<PathBuf>,
    plugin_path: &Path,
) -> Result<(), String> {
    let mut tasks = JoinSet::new();

    for master_plugin_file in master_plugin_files {
        let master_plugin_file = master_plugin_file.clone();
        let plugin_path = plugin_path.to_path_buf();
        tasks.spawn(async move {
            let context = format!(
                "Failed to load master plugin {} while collecting its asset references",
                master_plugin_file.display()
            );
            let master_plugin =
                load_plugin_with_context(master_plugin_file.clone(), context).await?;
            let master_asset_files =
                collect_asset_files(&master_plugin, &master_plugin_file, &plugin_path, false)
                    .await?;
            Ok::<_, String>((master_plugin_file, master_asset_files))
        });
    }

    while let Some(task_result) = tasks.join_next().await {
        let (master_plugin_file, master_asset_files) =
            task_result.map_err(|error| format!("Master asset scan task failed: {error}"))??;
        for master_asset_path in master_asset_files.asset_paths() {
            let master_asset_path = AssetPath {
                relative_path: master_asset_path,
            };
            if let Some(asset) = assets.lookup_asset(&master_asset_path)
                && asset.kind != Type::Plugin
                && assets.remove_asset(&asset)
            {
                info!(
                    "Excluding asset {} because it is referenced by master plugin {}.",
                    asset.path.relative_path.display(),
                    master_plugin_file.display()
                );
            }
        }
    }

    Ok(())
}

/// Returns an asset graph containing the plugin, its meshes, and their external textures.
pub async fn collect_asset_files(
    plugin: &Plugin,
    plugin_file: &Path,
    plugin_path: &Path,
    validate_references: bool,
) -> Result<AssetGraph, String> {
    let assets = AssetGraph::new();
    let plugin_name = plugin_file
        .file_name()
        .ok_or_else(|| format!("Plugin path has no file name: {}", plugin_file.display()))?;
    let plugin_asset = assets.add_root_asset(
        Type::Plugin,
        AssetPath {
            relative_path: PathBuf::from(plugin_name),
        },
    );
    let mut tasks = JoinSet::new();

    for object in plugin.objects_of_type::<Static>() {
        let mesh_path = Path::new("meshes").join(&object.mesh);
        let mesh_asset = assets.add_asset(
            Type::Mesh,
            AssetPath {
                relative_path: mesh_path.clone(),
            },
            &plugin_asset,
        );
        let plugin_file = plugin_file.to_path_buf();
        let plugin_path = plugin_path.to_path_buf();
        tasks.spawn_blocking(move || {
            let textures =
                scan_mesh_textures(&mesh_path, &plugin_file, &plugin_path, validate_references)?;
            Ok::<_, String>((mesh_asset, textures))
        });
    }

    while let Some(task_result) = tasks.join_next().await {
        let (mesh_asset, textures) =
            task_result.map_err(|error| format!("NIF scan task failed: {error}"))??;
        for texture_path in textures {
            assets.add_asset(
                Type::Texture,
                AssetPath {
                    relative_path: texture_path,
                },
                &mesh_asset,
            );
        }
    }

    assets.assert_valid();
    Ok(assets)
}
