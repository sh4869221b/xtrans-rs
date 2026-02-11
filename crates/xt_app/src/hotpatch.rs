#[cfg(all(debug_assertions, feature = "hotpatch", not(target_family = "wasm")))]
pub(crate) fn init_hotpatch() {
    use std::sync::Once;

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        dioxus_devtools::connect_subsecond();
        eprintln!("hotpatch: waiting for dx devserver");
    });
}

#[cfg(not(all(debug_assertions, feature = "hotpatch", not(target_family = "wasm"))))]
pub(crate) fn init_hotpatch() {}
