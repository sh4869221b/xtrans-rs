#[cfg(feature = "dioxus-desktop")]
mod dioxus_app;
#[cfg(feature = "egui-desktop")]
mod egui_app;

#[cfg(not(any(feature = "dioxus-desktop", feature = "egui-desktop")))]
compile_error!("Either feature \"egui-desktop\" or \"dioxus-desktop\" must be enabled.");

#[cfg(feature = "egui-desktop")]
fn main() {
    egui_app::run();
}

#[cfg(all(not(feature = "egui-desktop"), feature = "dioxus-desktop"))]
fn main() {
    dioxus_app::run();
}
