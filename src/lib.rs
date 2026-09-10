#![allow(clippy::mutable_key_type)]

use ::log::info;

mod args;
mod assets;
mod commands;
mod log;
mod update;

#[unsafe(no_mangle)]
pub extern "C" fn esp_tools_init() {
    log::init_logger();

    info!("ESP tools initialized");
}
