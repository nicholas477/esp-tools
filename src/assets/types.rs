use dashmap::DashSet;
use std::collections::hash_set::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

/// Asset path, always relative to a base path (usually the plugin directory)
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct AssetPath {
    pub relative_path: PathBuf,
}

impl AssetPath {
    pub fn make_full(&self, base_path: &Path) -> PathBuf {
        base_path.join(&self.relative_path)
    }

    pub fn exists(&self, base_path: &Path) -> bool {
        self.make_full(base_path).exists()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Mesh,
    Texture,
    Plugin,
}

#[derive(Clone)]
pub struct AssetRef(Weak<Asset>);

impl AssetRef {
    fn new(asset: &Arc<Asset>) -> Self {
        Self(Arc::downgrade(asset))
    }

    fn upgrade(&self) -> Option<Arc<Asset>> {
        self.0.upgrade()
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Hash for AssetRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl PartialEq for AssetRef {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl Eq for AssetRef {}

pub struct Asset {
    pub kind: Type,
    pub path: AssetPath,
    pub children: DashSet<AssetRef>,
    pub parents: DashSet<AssetRef>,
}

/// Adds a parent/child dependency between assets
pub fn add_dependency(parent: &Arc<Asset>, child: &Arc<Asset>) {
    parent.children.insert(AssetRef::new(child));
    child.parents.insert(AssetRef::new(parent));
}

impl Asset {
    pub fn new(kind: Type, path: AssetPath) -> Self {
        Asset {
            kind,
            path,
            children: DashSet::new(),
            parents: DashSet::new(),
        }
    }
}

impl Hash for Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Asset {}

pub struct AssetGraph {
    pub assets: DashSet<Arc<Asset>>,
}

impl AssetGraph {
    pub fn new() -> Self {
        AssetGraph {
            assets: DashSet::new(),
        }
    }

    pub fn add_root_asset(&self, kind: Type, path: AssetPath) -> Arc<Asset> {
        if let Some(asset) = self.lookup_asset(&path) {
            assert_eq!(asset.kind, kind);
            return asset;
        }

        let new_asset = Arc::new(Asset::new(kind, path));
        if self.assets.insert(Arc::clone(&new_asset)) {
            new_asset
        } else {
            self.lookup_asset(&new_asset.path)
                .expect("asset must exist after insertion")
        }
    }

    pub fn add_asset(&self, kind: Type, path: AssetPath, parent: &Arc<Asset>) -> Arc<Asset> {
        if let Some(asset) = self.lookup_asset(&path) {
            assert_eq!(asset.kind, kind);

            add_dependency(parent, &asset);
            asset
        } else {
            let asset = self.add_root_asset(kind, path);
            add_dependency(parent, &asset);
            asset
        }
    }

    pub fn remove_asset(&self, asset: &Arc<Asset>) -> bool {
        self.assets.remove(asset).is_some()
    }

    pub fn asset_paths(&self) -> Vec<PathBuf> {
        self.assets
            .iter()
            .map(|asset| asset.path.relative_path.clone())
            .collect()
    }

    pub fn lookup_asset(&self, path: &AssetPath) -> Option<Arc<Asset>> {
        self.assets
            .iter()
            .find(|asset| asset.path == *path)
            .map(|asset| Arc::clone(&asset))
    }

    // For each node on the graph, verify that all of its children have it as a parent, and all of its parents have it as a child.
    pub fn assert_valid(&self) {
        let mut visited_assets = HashSet::new();
        for asset in self.assets.iter() {
            if !visited_assets.insert(asset.path.clone()) {
                continue;
            }

            for child in asset.children.iter() {
                if let Some(child) = child.upgrade() {
                    assert!(
                        child
                            .parents
                            .iter()
                            .any(|parent| parent.ptr_eq(&AssetRef::new(&asset)))
                    );
                }
            }

            for parent in asset.parents.iter() {
                if let Some(parent) = parent.upgrade() {
                    assert!(
                        parent
                            .children
                            .iter()
                            .any(|child| child.ptr_eq(&AssetRef::new(&asset)))
                    );
                }
            }
        }
    }
}
