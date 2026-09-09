use std::path::Path;

use log::error;

use crate::assets;

use super::package::{self, EspFileGraph};

// Json schema for the Asset structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Asset {
    pub index: usize,
    pub name: String,
    pub export: bool,
    pub children: Vec<usize>,
}

impl TryFrom<crate::assets::AssetRef> for Asset {
    type Error = ();

    fn try_from(asset_ref: crate::assets::AssetRef) -> Result<Self, Self::Error> {
        if let Some((name, children)) =
            asset_ref.map_read(|asset| (asset.path.clone(), asset.children.clone()))
        {
            //let name = name.to_string();
            log::info!(
                "Creating Asset from AssetRef with name: {}",
                name.relative_path
                    .clone()
                    .into_string()
                    .unwrap_or("".into())
            );

            Ok(Self {
                index: asset_ref.index,
                name: name.relative_path
                    .clone()
                    .into_string().unwrap_or("".into()),
                export: true,
                children: children.into_iter().map(|child| child.index).collect(),
            })
        } else {
            Err(())
        }
    }
}

/// Scans an ESP file and returns an opaque graph pointer owned by the caller.
///
/// `input_file` must be a non-null pointer to `input_file_len_bytes` UTF-8 bytes. Returns null if
/// the path is invalid, the scan fails, or the scan panics.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn esp_tools_scan_esp_file_graph(
    input_file: *const u8,
    input_file_len_bytes: usize,
) -> *mut EspFileGraph {
    let result = std::panic::catch_unwind(|| {
        if input_file.is_null() {
            return Err("The input file path cannot be null".to_owned());
        }

        let input_file = unsafe { std::slice::from_raw_parts(input_file, input_file_len_bytes) };
        let input_file = std::str::from_utf8(input_file)
            .map_err(|error| format!("The input file path is not valid UTF-8: {error}"))?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Failed to start scan runtime: {error}"))?;

        runtime
            .block_on(package::scan_esp_file_graph(Path::new(input_file)))
            .map_err(|error| error.to_string())
    });

    match result {
        Ok(Ok(graph)) => Box::into_raw(Box::new(graph)),
        Ok(Err(scan_error)) => {
            error!("Failed to scan ESP file graph: {scan_error}");
            std::ptr::null_mut()
        }
        Err(_) => {
            error!("ESP file graph scan panicked");
            std::ptr::null_mut()
        }
    }
}

/// Releases a graph returned by [`esp_tools_scan_esp_file_graph`].
///
/// Passing null is allowed. Any non-null pointer must have been returned by the scan function and
/// not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn esp_tools_free_esp_file_graph(graph: *mut EspFileGraph) {
    if !graph.is_null() {
        unsafe {
            drop(Box::from_raw(graph));
        }
    }
}

// Json schema for the Asset structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileAssets {
    pub root_index: usize,
    pub assets: Vec<Asset>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn esp_tools_get_esp_file_assets_json(
    graph: *const EspFileGraph,
    json_bytes: *mut *mut u8,
    json_len_bytes: *mut usize,
) -> bool {
    if json_bytes.is_null() || json_len_bytes.is_null() {
        error!("The JSON output pointers cannot be null");
        return false;
    }

    unsafe {
        *json_bytes = std::ptr::null_mut();
        *json_len_bytes = 0;
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let graph = unsafe { graph.as_ref() }
            .ok_or_else(|| "The ESP file graph cannot be null".to_owned())?;

        let assets = package::get_esp_file_assets(&graph.plugin);
        let removed_assets = package::remove_master_file_assets(&mut assets.clone())
            .into_iter()
            .map(|asset_ref| asset_ref.index)
            .collect::<std::collections::HashSet<_>>();

        let mut assets_vec = Vec::new();
        for asset in &assets {
            if let Ok(mut asset) = Asset::try_from(asset.clone()) {
                asset.export =
                    graph.plugin.index == asset.index || !removed_assets.contains(&asset.index);
                assets_vec.push(asset);
            }
        }

        let file_assets = FileAssets {
            root_index: graph.plugin.index,
            assets: assets_vec,
        };

        Ok::<Box<[u8]>, String>(
            serde_json::to_string_pretty(&file_assets)
                .unwrap()
                .into_bytes()
                .into_boxed_slice(),
        )
    }));

    match result {
        Ok(Ok(json)) => {
            let json_len = json.len();
            let json_ptr = Box::into_raw(json).cast::<u8>();

            unsafe {
                *json_bytes = json_ptr;
                *json_len_bytes = json_len;
            }

            true
        }
        Ok(Err(serialization_error)) => {
            error!("Failed to serialize ESP file graph: {serialization_error}");
            false
        }
        Err(_) => {
            error!("ESP file graph JSON serialization panicked");
            false
        }
    }
}

/// Releases a UTF-8 byte buffer.
///
/// Passing null is allowed. Any non-null pointer and length must be the values returned by the JSON
/// getter and must not have been previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn esp_tools_free_utf8(value: *mut u8, value_len_bytes: usize) {
    if !value.is_null() {
        unsafe {
            let value = std::ptr::slice_from_raw_parts_mut(value, value_len_bytes);
            drop(Box::from_raw(value));
        }
    }
}
