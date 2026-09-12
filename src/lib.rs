#![allow(clippy::mutable_key_type)]

use ::log::info;

pub mod args;
pub mod assets;
pub mod commands;
pub mod log;
pub mod update;

#[unsafe(no_mangle)]
pub extern "C" fn esp_tools_init() {
    log::init_logger();

    info!("ESP tools initialized");
}
