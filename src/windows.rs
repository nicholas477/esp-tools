extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use nwg::NativeUi;

use crate::args::Commands::Package;
use crate::commands::package::{self, EspFileGraph};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum GuiAction {
    #[default]
    Cancel,
    Package,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ViewMode {
    #[default]
    Tree,
    FlatFiles,
}

#[derive(Default, nwd::NwgUi)]
pub struct EspTreeApp {
    #[nwg_control(
		size: (900, 650),
		position: (300, 300),
		title: "ESP Tools - Asset Tree",
		flags: "WINDOW|VISIBLE|MINIMIZE_BOX|MAXIMIZE_BOX|RESIZABLE"
	)]
    #[nwg_events(OnWindowClose: [EspTreeApp::exit], OnInit: [EspTreeApp::load_initial_plugin], OnResize: [EspTreeApp::resize_controls])]
    window: nwg::Window,

    initial_plugin: Option<PathBuf>,
    action: Cell<GuiAction>,
    view_mode: Cell<ViewMode>,
    is_loading: Cell<bool>,
    scan_receiver: RefCell<Option<Receiver<Result<EspFileGraph, String>>>>,
    scan_result: RefCell<Option<EspFileGraph>>,

    #[nwg_control(parent: window)]
    tree: nwg::TreeView,

    #[nwg_control(parent: window, flags: "VISIBLE|TAB_STOP")]
    file_list: nwg::ListBox<String>,

    #[nwg_control(text: "Loading asset references...", parent: window)]
    loading_label: nwg::Label,

    #[nwg_control(parent: window, flags: "VISIBLE|MARQUEE", marquee: true)]
    loading_progress: nwg::ProgressBar,

    #[nwg_control]
    #[nwg_events(OnNotice: [EspTreeApp::finish_loading])]
    scan_complete: nwg::Notice,

    #[nwg_control(text: "Tree", parent: window, size: (80, 32))]
    #[nwg_events(OnButtonClick: [EspTreeApp::show_tree_view])]
    tree_view_button: nwg::Button,

    #[nwg_control(text: "Files", parent: window, size: (80, 32))]
    #[nwg_events(OnButtonClick: [EspTreeApp::show_flat_file_view])]
    flat_file_view_button: nwg::Button,

    #[nwg_control(text: "Package Mod", parent: window, size: (120, 32))]
    #[nwg_events(OnButtonClick: [EspTreeApp::package_mod])]
    package_button: nwg::Button,

    #[nwg_control(text: "Cancel", parent: window, size: (120, 32))]
    #[nwg_events(OnButtonClick: [EspTreeApp::exit])]
    cancel_button: nwg::Button,
}

impl EspTreeApp {
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn package_mod(&self) {
        self.action.set(GuiAction::Package);
        nwg::stop_thread_dispatch();
    }

    fn load_initial_plugin(&self) {
        self.resize_controls();
        if let Some(plugin_path) = &self.initial_plugin {
            self.start_scan(plugin_path.clone());
        }
    }

    fn start_scan(&self, plugin_path: PathBuf) {
        self.is_loading.set(true);
        self.package_button.set_enabled(false);
        self.loading_label.set_visible(true);
        self.loading_progress.set_visible(true);

        let (sender, receiver) = mpsc::channel();
        *self.scan_receiver.borrow_mut() = Some(receiver);
        let notice_sender = self.scan_complete.sender();

        thread::spawn(move || {
            let result = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| format!("Failed to start scan runtime: {error}"))
                .and_then(|runtime| {
                    runtime
                        .block_on(package::scan_esp_file_graph(&plugin_path))
                        .map_err(|error| error.to_string())
                });
            let _ = sender.send(result);
            notice_sender.notice();
        });
    }

    fn finish_loading(&self) {
        let result = {
            let receiver = self.scan_receiver.borrow();
            let Some(receiver) = receiver.as_ref() else {
                return;
            };
            receiver.try_recv()
        };
        let Ok(result) = result else {
            return;
        };

        self.loading_label.set_visible(false);
        self.loading_progress.set_visible(false);
        self.is_loading.set(false);
        self.resize_controls();

        match result {
            Ok(graph) => {
                *self.scan_result.borrow_mut() = Some(graph);
                self.render_current_view();
                self.package_button.set_enabled(true);
            }
            Err(error) => {
                nwg::simple_message("ESP Tools", &error);
            }
        }
    }

    fn resize_controls(&self) {
        const MARGIN: u32 = 8;
        const BUTTON_WIDTH: u32 = 120;
        const BUTTON_HEIGHT: u32 = 32;
        const FOOTER_HEIGHT: u32 = BUTTON_HEIGHT + (MARGIN * 2);
        let loading_height = if self.is_loading.get() { 48 } else { 0 };

        let (width, height) = self.window.size();
        let tree_width = width.saturating_sub(MARGIN * 2);
        let tree_height = height.saturating_sub(FOOTER_HEIGHT + loading_height);
        let button_y = height.saturating_sub(BUTTON_HEIGHT + MARGIN);
        let cancel_x = width.saturating_sub(BUTTON_WIDTH + MARGIN);
        let package_x = cancel_x.saturating_sub(BUTTON_WIDTH + MARGIN);

        self.loading_label
            .set_position(MARGIN as i32, MARGIN as i32);
        self.loading_label.set_size(tree_width, 20);
        self.loading_progress
            .set_position(MARGIN as i32, (MARGIN + 24) as i32);
        self.loading_progress.set_size(tree_width, 16);
        self.tree
            .set_position(MARGIN as i32, (MARGIN + loading_height) as i32);
        self.tree.set_size(tree_width, tree_height);
        self.file_list
            .set_position(MARGIN as i32, (MARGIN + loading_height) as i32);
        self.file_list.set_size(tree_width, tree_height);
        self.tree_view_button
            .set_position(MARGIN as i32, button_y as i32);
        self.flat_file_view_button
            .set_position((MARGIN + 88) as i32, button_y as i32);
        self.package_button
            .set_position(package_x as i32, button_y as i32);
        self.cancel_button
            .set_position(cancel_x as i32, button_y as i32);
    }

    fn show_tree_view(&self) {
        self.view_mode.set(ViewMode::Tree);
        self.render_current_view();
    }

    fn show_flat_file_view(&self) {
        self.view_mode.set(ViewMode::FlatFiles);
        self.render_current_view();
    }

    fn render_current_view(&self) {
        let plugin = self
            .scan_result
            .borrow()
            .as_ref()
            .map(|graph| graph.plugin.clone());
        let Some(plugin) = plugin else {
            return;
        };

        match self.view_mode.get() {
            ViewMode::Tree => {
                self.file_list.set_visible(false);
                self.tree.set_visible(true);
                self.show_plugin(&plugin);
            }
            ViewMode::FlatFiles => {
                self.tree.set_visible(false);
                self.file_list.set_visible(true);
                self.show_flat_files(&plugin);
            }
        }
    }

    fn show_flat_files(&self, plugin: &crate::assets::AssetRef) {
        let mut assets = package::get_esp_file_assets(plugin)
            .into_iter()
            .filter_map(|asset| {
                asset
                    .upgrade()
                    .map(|node| node.asset.path.relative_path.display().to_string())
            })
            .collect::<Vec<_>>();
        assets.sort();
        self.file_list.set_collection(assets);
    }
    fn show_plugin(&self, plugin: &crate::assets::AssetRef) {
        self.tree.clear();
        let Some(plugin_node) = plugin.upgrade() else {
            nwg::simple_message("ESP Tools", "The scanned plugin was dropped.");
            return;
        };

        let plugin_item = self.tree.insert_item(
            &plugin_node.asset.path.relative_path.display().to_string(),
            None,
            nwg::TreeInsert::Root,
        );
        self.insert_children(plugin, &plugin_item, true);

        for item in self.tree.iter() {
            self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
        }
    }

    fn insert_children(
        &self,
        parent: &crate::assets::AssetRef,
        parent_item: &nwg::TreeItem,
        exclude_plugin_children: bool,
    ) {
        let mut children = parent.children(false).into_iter().collect::<Vec<_>>();
        children.sort();

        for child in children {
            if let Some(child_node) = child.upgrade() {
                if exclude_plugin_children && child_node.asset.kind == crate::assets::Type::Plugin {
                    continue;
                }
                let child_item = self.tree.insert_item(
                    &child_node.asset.path.relative_path.display().to_string(),
                    Some(parent_item),
                    nwg::TreeInsert::Last,
                );
                self.insert_children(&child, &child_item, false);
            }
        }
    }
}

pub fn run(args: &crate::args::Args) -> Result<(), Box<dyn std::error::Error>> {
    if let Package(args) = &args.command {
        nwg::init()?;
        nwg::Font::set_global_family("Segoe UI")?;

        let app = EspTreeApp::build_ui(EspTreeApp {
            initial_plugin: Some(args.file.clone()),
            ..Default::default()
        })?;
        let mut package_tooltip = nwg::Tooltip::default();
        nwg::Tooltip::builder()
            .register(
                &app.package_button,
                "Create a ZIP containing this plugin, its models, and textures.",
            )
            .build(&mut package_tooltip)?;
        nwg::dispatch_thread_events();
    }

    Ok(())
}
