use dashmap::DashSet;
use log::{error, info};
use std::cmp::Ordering;
use std::collections::hash_set::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use tes3::esp::{Plugin, Static};
use tes3::nif::TextureSource::External;
use tes3::nif::{NiSourceTexture, NiStream};

pub type Set<T> = DashSet<T>;
pub type Map<K, V> = dashmap::DashMap<K, V>;

/// Asset path, always relative to a base path (usually the plugin directory)
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
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

impl fmt::Debug for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.relative_path.display())
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.relative_path.display())
    }
}

#[derive(Clone)]
pub struct AssetRef(Weak<AssetNode>);

impl AssetRef {
    pub fn new(asset: &Arc<AssetNode>) -> Self {
        Self(Arc::downgrade(asset))
    }

    /// Returns a set of child assets for this asset. If `include_descendents` is true, it will also include all descendents recursively.
    pub fn children(&self, include_descendents: bool) -> HashSet<AssetRef> {
        self.upgrade()
            .map(|asset| asset.children(include_descendents))
            .unwrap_or_default()
    }

    /// Returns a set of parent assets for this asset. If `include_ancestors` is true, it will also include all ancestors recursively.
    pub fn parents(&self, include_ancestors: bool) -> HashSet<AssetRef> {
        self.upgrade()
            .map(|asset| asset.parents(include_ancestors))
            .unwrap_or_default()
    }

    pub fn upgrade(&self) -> Option<Arc<AssetNode>> {
        self.0.upgrade()
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl fmt::Display for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(asset) = self.upgrade() {
            write!(f, "AssetRef({:?})", asset)
        } else {
            write!(f, "AssetRef(Dropped)")
        }
    }
}

impl fmt::Debug for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(asset) = self.upgrade() {
            write!(f, "AssetRef({:?})", asset)
        } else {
            write!(f, "AssetRef(Dropped)")
        }
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

impl PartialOrd for AssetRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.upgrade()
                .unwrap()
                .path
                .cmp(&other.upgrade().unwrap().path),
        )
    }
}

impl Ord for AssetRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.upgrade()
            .unwrap()
            .path
            .cmp(&other.upgrade().unwrap().path)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Mesh,
    Texture,
    Plugin,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub kind: Type,
    pub path: AssetPath,
}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Asset {}

impl Hash for Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

/// A node in the asset graph, representing an asset and its relationships to other assets.
pub struct AssetNode {
    pub asset: Asset,
    pub children: Set<AssetRef>,
    pub parents: Set<AssetRef>,
}

impl AssetNode {
    pub fn new(asset: Asset) -> Self {
        AssetNode {
            asset,
            children: Set::new(),
            parents: Set::new(),
        }
    }

    /// Returns a set of child assets for this asset. If `include_descendents` is true, it will also include all descendents recursively.
    pub fn children(&self, include_descendents: bool) -> HashSet<AssetRef> {
        let mut children = HashSet::new();
        for child in self.children.iter() {
            children.insert(child.clone());
            if include_descendents {
                children.extend(child.children(true));
            }
        }
        children
    }

    /// Returns a set of parent assets for this asset. If `include_ancestors` is true, it will also include all ancestors recursively.
    pub fn parents(&self, include_ancestors: bool) -> HashSet<AssetRef> {
        let mut parents = HashSet::new();
        for parent in self.parents.iter() {
            parents.insert(parent.clone());
            if include_ancestors {
                parents.extend(parent.parents(true));
            }
        }
        parents
    }
}

impl Deref for AssetNode {
    type Target = Asset;

    fn deref(&self) -> &Self::Target {
        &self.asset
    }
}

impl fmt::Display for AssetNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetNode(asset: {:?})", self.asset)
    }
}

impl fmt::Debug for AssetNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetNode(asset: {:?})", self.asset)
    }
}

impl PartialEq for AssetNode {
    fn eq(&self, other: &Self) -> bool {
        self.asset == other.asset
    }
}

impl Eq for AssetNode {}

impl Hash for AssetNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.asset.hash(state);
    }
}

/// Adds a parent/child dependency between assets
pub fn add_dependency(parent: &AssetRef, child: &AssetRef) {
    let Some(parent) = parent.upgrade() else {
        error!("Failed to add dependency: parent asset has been dropped");
        return;
    };

    let Some(child) = child.upgrade() else {
        error!("Failed to add dependency: child asset has been dropped");
        return;
    };

    parent.children.insert(AssetRef::new(&child));
    child.parents.insert(AssetRef::new(&parent));
}

impl Asset {
    pub fn new(kind: Type, path: AssetPath) -> Self {
        Asset { kind, path }
    }

    // Loads the asset file, returns its children as a set of assets.
    pub fn load_children(&self, base_path: &Path) -> Set<Asset> {
        let children = Set::new();

        match self.kind {
            // Meshes have textures as children
            Type::Mesh => {
                let mesh_path = self.path.make_full(base_path);
                let mut stream = NiStream::new();

                if stream.load_path(&mesh_path).is_err() {
                    error!("Failed to load mesh: \"{}\"", mesh_path.display());
                    return children;
                }

                for object in stream.objects_of_type::<NiSourceTexture>() {
                    if let External(file_name) = &object.source {
                        children.insert(Asset {
                            path: AssetPath {
                                relative_path: PathBuf::from(file_name),
                            },
                            kind: Type::Texture,
                        });
                    }
                }
            }

            // Textures have no children
            Type::Texture => {
                return children;
            }

            // Plugins have esp files and meshes as children
            Type::Plugin => {
                let plugin_path = self.path.make_full(base_path);

                if let Ok(plugin) = Plugin::from_path(&plugin_path) {
                    // Add meshes as children
                    for object in plugin.objects_of_type::<Static>() {
                        let mesh_path = Path::new("meshes").join(&object.mesh);
                        children.insert(Asset {
                            path: AssetPath {
                                relative_path: PathBuf::from(mesh_path),
                            },
                            kind: Type::Mesh,
                        });
                    }

                    // Add masters
                    if let Some(header) = plugin.header() {
                        for master in &header.masters {
                            children.insert(Asset {
                                path: AssetPath {
                                    relative_path: PathBuf::from(&master.0),
                                },
                                kind: Type::Plugin,
                            });
                        }
                    } else {
                        error!(
                            "Failed to load plugin header: \"{}\"",
                            plugin_path.display()
                        );
                    }
                } else {
                    error!(
                        "Failed to load plugin header: \"{}\"",
                        plugin_path.display()
                    );
                }
            }
        }

        children
    }
}

#[derive(Clone)]
pub struct AssetGraph {
    pub nodes: Arc<Map<AssetPath, Arc<AssetNode>>>,
}

impl AssetGraph {
    pub fn new() -> Self {
        AssetGraph {
            nodes: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub fn new_with_root(root_asset: &Asset) -> (Self, AssetRef) {
        let graph = AssetGraph::new();
        let new_asset = graph.add_asset(&root_asset.kind, &root_asset.path, None);
        (graph, new_asset)
    }

    pub fn add_asset(&self, kind: &Type, path: &AssetPath, parent: Option<AssetRef>) -> AssetRef {
        // Atomically get or insert the asset node, and return a reference to it.
        let asset = self
            .nodes
            .entry(path.clone())
            .or_insert_with(|| Arc::new(AssetNode::new(Asset::new(kind.clone(), path.clone()))));

        assert_eq!(&asset.asset.kind, kind);

        let asset_ref = AssetRef::new(&asset);
        if let Some(parent) = parent {
            add_dependency(&parent, &asset_ref);
        }

        asset_ref
    }

    pub fn remove_asset(&self, path: &AssetPath) -> bool {
        self.nodes.remove(path).is_some()
    }

    pub fn lookup_asset<'a>(&'a self, path: &AssetPath) -> Option<AssetRef> {
        self.nodes.get(path).map(|asset| AssetRef::new(&asset))
    }

    // For each node on the graph, verify that all of its children have it as a parent, and all of its parents have it as a child.
    pub fn assert_valid(&self) {
        let mut visited_assets = HashSet::new();
        for node in self.nodes.iter() {
            if !visited_assets.insert(node.asset.path.clone()) {
                continue;
            }

            for child in node.children.iter() {
                if let Some(child) = child.upgrade() {
                    assert!(
                        child
                            .parents
                            .iter()
                            .any(|parent| parent.ptr_eq(&AssetRef::new(&node)))
                    );
                }
            }

            for parent in node.parents.iter() {
                if let Some(parent) = parent.upgrade() {
                    assert!(
                        parent
                            .children
                            .iter()
                            .any(|child| child.ptr_eq(&AssetRef::new(&node)))
                    );
                }
            }
        }
    }
}
