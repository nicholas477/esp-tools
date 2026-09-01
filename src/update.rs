use log::info;
use self_update::cargo_crate_version;

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("nicholas477")
        .repo_name("esp-tools")
        .bin_name("esp-tools")
        .show_output(false)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    if status.is_up_to_date() {
        info!("Already up to date!");
    } else {
        info!("Updated to version: {}", status.version());
    }
    Ok(())
}
