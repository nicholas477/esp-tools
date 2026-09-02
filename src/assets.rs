use dashmap::DashSet;
use log::info;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;

mod types;
pub use types::*;

/// Walks all asset references (by loading files) recursively and collects them in a graph
#[async_recursion::async_recursion]
pub async fn collect_references(
    graph: &AssetGraph,
    asset: &AssetRef,
    base_path: &Path,
    scanned_assets: &Arc<DashSet<AssetPath>>,
) {
    let mut tasks = JoinSet::new();

    let children = asset.upgrade().unwrap().asset.load_children(base_path);

    for child in children {
        let parent = asset.clone();
        let graph = graph.clone();
        let scanned_assets = scanned_assets.clone();
        let base_path = base_path.to_path_buf();

        tasks.spawn(async move {
            let child_ref = graph.add_asset(&child.kind, &child.path, Some(parent));
            if scanned_assets.insert(child.path.clone()) {
                collect_references(&graph, &child_ref, &base_path, &scanned_assets).await;
            }
        });
    }

    tasks.join_all().await;
}
