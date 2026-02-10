use xt_app::driver::AppDriver;
use xt_app::state::Tab;

#[test]
fn e2e_boot_001_initial_state_is_empty() {
    let driver = AppDriver::new();
    let snapshot = driver.snapshot();

    assert_eq!(snapshot.total_entries, 0);
    assert_eq!(snapshot.translated_entries, 0);
    assert_eq!(snapshot.selected_key, None);
    assert_eq!(snapshot.active_tab, Tab::Home);
}
