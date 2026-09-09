use dashmap::DashSet;
use log::error;
use std::cmp::Ordering;
use std::collections::hash_set::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tes3::esp::{Plugin, Static};
use tes3::nif::TextureSource::External;
use tes3::nif::{NiSourceTexture, NiStream};

pub type Set<T> = DashSet<T>;
pub type Map<K, V> = dashmap::DashMap<K, V>;

/// Asset path, always relative to a base path (usually the plugin directory)
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetPath {
    pub relative_path: PathBuf,
}

#[allow(dead_code)]
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetRef {
    pub index: usize,

    #[serde(skip_serializing, skip_deserializing)]
    pub nodes: Arc<RwLock<Vec<AssetNode>>>,
}

impl AssetRef {
    pub fn new(index: usize, nodes: Arc<RwLock<Vec<AssetNode>>>) -> Self {
        Self { index, nodes }
    }

    /// Returns a set of child assets for this asset. If `include_descendents` is true, it will also include all descendents recursively.
    pub fn children(&self, include_descendents: bool) -> HashSet<AssetRef> {
        self.nodes
            .read()
            .unwrap()
            .get(self.index)
            .map(|asset| asset.children(include_descendents))
            .unwrap_or_default()
    }

    /// Returns a set of parent assets for this asset. If `include_ancestors` is true, it will also include all ancestors recursively.
    pub fn parents(&self, include_ancestors: bool) -> HashSet<AssetRef> {
        self.nodes
            .read()
            .unwrap()
            .get(self.index)
            .map(|asset| asset.parents(include_ancestors))
            .unwrap_or_default()
    }

    pub fn get_copy(&self) -> Option<AssetNode> {
        self.nodes.read().unwrap().get(self.index).cloned()
    }

    pub fn map_read<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&AssetNode) -> R,
    {
        self.nodes.read().unwrap().get(self.index).map(f)
    }

    pub fn map_write<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut AssetNode) -> R,
    {
        self.nodes.write().unwrap().get_mut(self.index).map(f)
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.index == other.index && Arc::ptr_eq(&self.nodes, &other.nodes)
    }
}

impl fmt::Display for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(asset) = self.get_copy() {
            write!(f, "AssetRef({asset:?})")
        } else {
            write!(f, "AssetRef(Dropped)")
        }
    }
}

impl fmt::Debug for AssetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(asset) = self.get_copy() {
            write!(f, "AssetRef({asset:?})")
        } else {
            write!(f, "AssetRef(Dropped)")
        }
    }
}

impl Hash for AssetRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.nodes).hash(state);
        self.index.hash(state);
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
        Some(self.cmp(other))
    }
}

impl Ord for AssetRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_copy()
            .unwrap()
            .path
            .cmp(&other.get_copy().unwrap().path)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Type {
    Mesh,
    Texture,
    Plugin,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetNode {
    pub asset: Asset,
    pub children: Set<AssetRef>,

    #[serde(skip_serializing, skip_deserializing)]
    pub parents: Set<AssetRef>,
}

#[allow(dead_code)]
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
    if parent
        .map_write(|parent_node| parent_node.children.insert(child.clone()))
        .is_none()
    {
        error!("Failed to add dependency: parent asset has been dropped");
        return;
    }

    if child
        .map_write(|child_node| child_node.parents.insert(parent.clone()))
        .is_none()
    {
        error!("Failed to add dependency: child asset has been dropped");
        return;
    }
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
                    //error!("Failed to load mesh: \"{}\"", mesh_path.display());
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
                                relative_path: mesh_path,
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

/// A graph of assets, where each asset can have multiple children and parents.
///
/// I'm not really happy with the way I've implemented this, the memory fragmentation must be insane tbh.
/// The speed isn't so bad. The only slow operation is Drop, and we can just ignore that by dropping it in another thread.
#[derive(Clone)]
pub struct AssetGraph {
    pub nodes: Arc<Map<AssetPath, AssetRef>>,
    pub nodes_by_index: Arc<RwLock<Vec<AssetNode>>>,
}

#[allow(dead_code)]
impl AssetGraph {
    pub fn new() -> Self {
        AssetGraph {
            nodes: Arc::new(dashmap::DashMap::new()),
            nodes_by_index: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn new_with_root(root_asset: &Asset) -> (Self, AssetRef) {
        let graph = AssetGraph::new();
        let new_asset = graph.add_asset(&root_asset.kind, &root_asset.path, None);
        (graph, new_asset)
    }

    pub fn add_asset(&self, kind: &Type, path: &AssetPath, parent: Option<AssetRef>) -> AssetRef {
        let nodes = self.nodes_by_index.clone();
        let asset_ref = self
            .nodes
            .entry(path.clone())
            .or_insert_with(|| {
                let mut nodes_by_index = nodes.write().unwrap();
                let index = nodes_by_index.len();
                nodes_by_index.push(AssetNode::new(Asset::new(kind.clone(), path.clone())));
                AssetRef::new(index, nodes.clone())
            })
            .clone();

        assert_eq!(
            asset_ref.map_read(|node| node.asset.kind.clone()).as_ref(),
            Some(kind)
        );

        if let Some(parent) = parent {
            add_dependency(&parent, &asset_ref);
        }

        asset_ref
    }

    pub fn remove_asset(&self, path: &AssetPath) -> bool {
        self.nodes.remove(path).is_some()
    }

    pub fn lookup_asset(&self, path: &AssetPath) -> Option<AssetRef> {
        self.nodes.get(path).map(|asset| asset.clone())
    }

    // For each node on the graph, verify that all of its children have it as a parent, and all of its parents have it as a child.
    pub fn assert_valid(&self) {
        let mut visited_assets = HashSet::new();
        for node in self.nodes.iter() {
            let node = node.value();
            let Some(node_copy) = node.get_copy() else {
                continue;
            };
            if !visited_assets.insert(node_copy.path.clone()) {
                continue;
            }

            for child in node_copy.children.iter() {
                assert!(
                    child
                        .map_read(|child_node| child_node
                            .parents
                            .iter()
                            .any(|parent| parent.ptr_eq(node)))
                        .unwrap_or(false)
                );
            }

            for parent in node_copy.parents.iter() {
                assert!(
                    parent
                        .map_read(|parent_node| parent_node
                            .children
                            .iter()
                            .any(|child| child.ptr_eq(node)))
                        .unwrap_or(false)
                );
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self.nodes_by_index.read().unwrap().clone()).unwrap()
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.nodes_by_index.read().unwrap().clone()).unwrap()
    }

    pub fn to_pretty_json_string(&self) -> String {
        serde_json::to_string_pretty(&self.nodes_by_index.read().unwrap().clone()).unwrap()
    }
}
