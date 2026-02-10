use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xt_app::actions::AppAction;
use xt_app::driver::AppDriver;
use xt_core::formats::strings::{read_strings, write_strings, StringsEntry, StringsFile};
use xt_core::import_export::export_entries;
use xt_core::model::Entry;

#[test]
fn e2e_io_str_001_load_edit_save_round_trip() {
    let root = test_temp_dir("io_str");
    let input = root.join("weapons_english.strings");
    let out = root.join("weapons_english_translated.strings");

    write_strings_file(
        &input,
        StringsFile {
            entries: vec![
                StringsEntry {
                    id: 1,
                    text: "Iron Sword".to_string(),
                },
                StringsEntry {
                    id: 2,
                    text: "Steel Sword".to_string(),
                },
            ],
        },
    );

    let mut driver = AppDriver::new();
    driver
        .dispatch(AppAction::LoadStrings(input))
        .expect("load strings");
    driver
        .dispatch(AppAction::SelectEntry("strings:1".to_string()))
        .expect("select");
    driver
        .dispatch(AppAction::SetEditTarget("鉄の剣".to_string()))
        .expect("set target");
    driver.dispatch(AppAction::ApplyEdit).expect("apply");
    driver
        .dispatch(AppAction::SaveAsPath(out.clone()))
        .expect("save as");

    let saved = std::fs::read(&out).expect("read out file");
    let parsed = read_strings(&saved).expect("parse out strings");
    assert_eq!(parsed.entries[0].text, "鉄の剣");
    assert_eq!(parsed.entries[1].text, "Steel Sword");
}

#[test]
fn e2e_xml_001_apply_from_editor_updates_target() {
    let root = test_temp_dir("xml_apply");
    let input = root.join("armor_english.strings");

    write_strings_file(
        &input,
        StringsFile {
            entries: vec![StringsEntry {
                id: 7,
                text: "Iron Armor".to_string(),
            }],
        },
    );

    let mut driver = AppDriver::new();
    driver
        .dispatch(AppAction::LoadStrings(input))
        .expect("load strings");

    let xml = export_entries(&[Entry {
        key: "strings:7".to_string(),
        source_text: "Iron Armor".to_string(),
        target_text: "鉄の鎧".to_string(),
    }]);

    driver
        .dispatch(AppAction::SetXmlText(xml))
        .expect("set xml text");
    driver
        .dispatch(AppAction::ApplyXmlFromEditor)
        .expect("apply xml from editor");

    let target = driver
        .state()
        .entries()
        .iter()
        .find(|entry| entry.key == "strings:7")
        .map(|entry| entry.target_text.clone())
        .expect("entry exists");
    assert_eq!(target, "鉄の鎧");
    assert!(driver.state().file_status.contains("updated=1"));
}

#[test]
fn e2e_dict_001_build_and_quick_auto_selection() {
    let root = test_temp_dir("dict_quick");
    let dict_dir = root.join("dict");
    std::fs::create_dir_all(&dict_dir).expect("create dict dir");

    write_strings_file(
        &dict_dir.join("skyrim_english.strings"),
        StringsFile {
            entries: vec![StringsEntry {
                id: 9,
                text: "Steel Shield".to_string(),
            }],
        },
    );
    write_strings_file(
        &dict_dir.join("skyrim_japanese.strings"),
        StringsFile {
            entries: vec![StringsEntry {
                id: 9,
                text: "鋼鉄の盾".to_string(),
            }],
        },
    );

    let input = root.join("runtime_english.strings");
    write_strings_file(
        &input,
        StringsFile {
            entries: vec![StringsEntry {
                id: 9,
                text: "Steel Shield".to_string(),
            }],
        },
    );

    let mut driver = AppDriver::new();
    driver
        .dispatch(AppAction::LoadStrings(input))
        .expect("load strings");
    driver
        .dispatch(AppAction::SelectEntry("strings:9".to_string()))
        .expect("select");
    driver
        .dispatch(AppAction::SetDictRoot(
            dict_dir.to_string_lossy().into_owned(),
        ))
        .expect("set dict root");
    driver
        .dispatch(AppAction::BuildDictionary)
        .expect("build dictionary");
    driver.dispatch(AppAction::QuickAuto).expect("quick auto");

    let target = driver
        .state()
        .entries()
        .iter()
        .find(|entry| entry.key == "strings:9")
        .map(|entry| entry.target_text.clone())
        .expect("entry exists");
    assert_eq!(target, "鋼鉄の盾");
}

fn write_strings_file(path: &Path, strings: StringsFile) {
    let bytes = write_strings(&strings).expect("encode strings");
    std::fs::write(path, bytes).expect("write strings");
}

fn test_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "xt_app_e2e_{}_{}_{}",
        prefix,
        std::process::id(),
        stamp
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
