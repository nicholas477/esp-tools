extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nwg::NativeUi;
use tes3::esp::{Plugin, Static};
use tes3::nif::TextureSource::External;
use tes3::nif::{NiSourceTexture, NiStream};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum GuiAction {
    #[default]
    Cancel,
    Package,
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

    #[nwg_control(parent: window)]
    tree: nwg::TreeView,

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
            self.show_plugin(plugin_path);
        }
    }

    fn resize_controls(&self) {
        const MARGIN: u32 = 8;
        const BUTTON_WIDTH: u32 = 120;
        const BUTTON_HEIGHT: u32 = 32;
        const FOOTER_HEIGHT: u32 = BUTTON_HEIGHT + (MARGIN * 2);

        let (width, height) = self.window.size();
        let tree_width = width.saturating_sub(MARGIN * 2);
        let tree_height = height.saturating_sub(FOOTER_HEIGHT + MARGIN);
        let button_y = height.saturating_sub(BUTTON_HEIGHT + MARGIN);
        let cancel_x = width.saturating_sub(BUTTON_WIDTH + MARGIN);
        let package_x = cancel_x.saturating_sub(BUTTON_WIDTH + MARGIN);

        self.tree.set_position(MARGIN as i32, MARGIN as i32);
        self.tree.set_size(tree_width, tree_height);
        self.package_button
            .set_position(package_x as i32, button_y as i32);
        self.cancel_button
            .set_position(cancel_x as i32, button_y as i32);
    }

    fn show_plugin(&self, plugin_path: &Path) {
        match collect_model_textures(plugin_path) {
            Ok(models) => {
                self.tree.clear();
                let plugin_item = self.tree.insert_item(
                    &plugin_path.display().to_string(),
                    None,
                    nwg::TreeInsert::Root,
                );

                for (model_path, textures) in models {
                    let model_item = self.tree.insert_item(
                        &model_path,
                        Some(&plugin_item),
                        nwg::TreeInsert::Last,
                    );
                    for texture_path in textures {
                        self.tree.insert_item(
                            &texture_path,
                            Some(&model_item),
                            nwg::TreeInsert::Last,
                        );
                    }
                }

                for item in self.tree.iter() {
                    self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
                }
            }
            Err(error) => {
                nwg::simple_message("ESP Tools", &error);
            }
        }
    }
}

fn collect_model_textures(
    plugin_path: &Path,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let plugin = Plugin::from_path(plugin_path)
        .map_err(|error| format!("Failed to load {}: {error}", plugin_path.display()))?;
    let plugin_directory = plugin_path.parent().ok_or_else(|| {
        format!(
            "Unable to determine the folder for {}",
            plugin_path.display()
        )
    })?;
    let mut models = BTreeMap::new();

    for object in plugin.objects_of_type::<Static>() {
        let model_path = Path::new("meshes").join(&object.mesh);
        let model_name = model_path.to_string_lossy().into_owned();
        let textures = models.entry(model_name).or_insert_with(BTreeSet::new);
        let full_model_path = plugin_directory.join(&model_path);
        let mut stream = NiStream::new();

        if stream.load_path(&full_model_path).is_err() {
            continue;
        }

        for texture in stream.objects_of_type::<NiSourceTexture>() {
            if let External(texture_path) = &texture.source {
                textures.insert(Path::new(texture_path).to_string_lossy().into_owned());
            }
        }
    }

    Ok(models)
}

pub fn run(plugin_path: &Path) -> Result<GuiAction, Box<dyn std::error::Error>> {
    nwg::init()?;
    nwg::Font::set_global_family("Segoe UI")?;

    let app = EspTreeApp::build_ui(EspTreeApp {
        initial_plugin: Some(plugin_path.to_path_buf()),
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
    Ok(app.action.get())
}
