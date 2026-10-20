//! Interface Leptos de Liquid Compagnon, compilée en WebAssembly par Trunk.

mod api;
mod app;
mod charts;
mod components;
mod files;
mod format;
mod icons;
mod pages;
mod shell;
mod state;
mod util;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
