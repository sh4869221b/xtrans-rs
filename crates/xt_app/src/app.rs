use eframe::egui::{self, Align, Layout, RichText, ScrollArea, TextEdit, TopBottomPanel};

use crate::actions::{dispatch, AppAction};
use crate::state::{row_fields, AppState, Tab};

pub fn launch() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "xtrans-rs",
        options,
        Box::new(|_cc| Ok(Box::new(XtransApp::default()))),
    )
}

#[derive(Default)]
pub struct XtransApp {
    state: AppState,
}

impl XtransApp {
    fn run_action(&mut self, action: AppAction) {
        if let Err(err) = dispatch(&mut self.state, action) {
            if self.state.file_status.is_empty() {
                self.state.file_status = err;
            }
        }
    }

    fn draw_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("ファイル", |ui| {
                if ui.button("Stringsファイルを開く").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Strings", &["strings", "dlstrings", "ilstrings"])
                        .pick_file()
                    {
                        self.run_action(AppAction::LoadStrings(path));
                    }
                }
                if ui.button("Esp/Esmファイルを開く").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Plugin", &["esp", "esm", "esl", "xtplugin"])
                        .pick_file()
                    {
                        self.run_action(AppAction::LoadPlugin(path));
                    }
                }
                if ui.button("翻訳XMLを開く").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("XML", &["xml"])
                        .pick_file()
                    {
                        self.run_action(AppAction::LoadXml(path));
                    }
                }
                if ui.button("翻訳XMLを書き出し").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::ExportXmlToEditor);
                }
                if ui.button("上書き保存").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::SaveOverwrite);
                }
                if ui.button("別名保存").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        self.run_action(AppAction::SaveAsPath(path));
                    } else {
                        self.run_action(AppAction::SaveAsAuto);
                    }
                }
            });

            ui.menu_button("翻訳", |ui| {
                if ui.button("辞書を構築").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::BuildDictionary);
                }
                if ui.button("Quick自動翻訳 (Ctrl-R)").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::QuickAuto);
                }
            });

            ui.menu_button("オプション", |ui| {
                if ui.button("言語タブを開く").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::SetActiveTab(Tab::Lang));
                }
                if ui.button("言語ペアを既定に戻す").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::ResetDictLanguagePair);
                }
            });

            ui.menu_button("ツール", |ui| {
                if ui.button("Undo").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::Undo);
                }
                if ui.button("Redo").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::Redo);
                }
                if ui.button("ログタブを開く").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::SetActiveTab(Tab::Log));
                }
            });
        });
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("検索");
            let mut query = self.state.pane.query().to_string();
            if ui
                .add(TextEdit::singleline(&mut query).desired_width(280.0))
                .changed()
            {
                self.run_action(AppAction::SetQuery(query));
            }

            if ui.button("Validate").clicked() {
                self.run_action(AppAction::Validate);
            }
            if ui.button("Diff").clicked() {
                self.run_action(AppAction::DiffCheck);
            }
            if ui.button("Encoding").clicked() {
                self.run_action(AppAction::EncodingCheck);
            }
            if ui.button("Build Hybrid").clicked() {
                self.run_action(AppAction::BuildHybrid);
            }
        });

        let counts = self.state.channel_counts();
        let ratio = self.state.translation_ratio() / 100.0;
        ui.horizontal(|ui| {
            ui.label(format!(
                "STRINGS [{}/{}]",
                counts.translated, counts.strings
            ));
            ui.add(egui::ProgressBar::new(ratio).desired_width(140.0));
            ui.label(format!("DLSTRINGS [0/{}]", counts.dlstrings));
            ui.label(format!("ILSTRINGS [0/{}]", counts.ilstrings));
        });
    }

    fn draw_entry_list(&mut self, ui: &mut egui::Ui) {
        let filtered = self.state.filtered_entries();
        ui.heading("Entries");
        ui.separator();

        ScrollArea::vertical().show_rows(ui, 22.0, filtered.len(), |ui, row_range| {
            for row in row_range {
                let entry = &filtered[row];
                let selected = self.state.selected_key().as_deref() == Some(entry.key.as_str());
                let (edid, record_id, ld) = row_fields(&entry.key, &entry.target_text);
                let label = format!(
                    "{} | {} | {} | {} | {}",
                    edid, record_id, entry.source_text, entry.target_text, ld
                );

                if ui.selectable_label(selected, label).clicked() {
                    self.run_action(AppAction::SelectEntry(entry.key.clone()));
                }
            }
        });
    }

    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in Tab::all() {
                let selected = self.state.active_tab == tab;
                if ui.selectable_label(selected, label).clicked() {
                    self.run_action(AppAction::SetActiveTab(tab));
                }
            }
        });
    }

    fn draw_home_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(entry) = self.state.selected_entry() {
            ui.label(format!("Key: {}", entry.key));

            let mut source = self.state.edit_source.clone();
            if ui
                .add(
                    TextEdit::multiline(&mut source)
                        .desired_rows(4)
                        .hint_text("原文"),
                )
                .changed()
            {
                self.run_action(AppAction::SetEditSource(source));
            }

            let mut target = self.state.edit_target.clone();
            if ui
                .add(
                    TextEdit::multiline(&mut target)
                        .desired_rows(4)
                        .hint_text("訳文"),
                )
                .changed()
            {
                self.run_action(AppAction::SetEditTarget(target));
            }

            ui.horizontal(|ui| {
                if ui.button("Apply Edit").clicked() {
                    self.run_action(AppAction::ApplyEdit);
                }
                if ui.button("Quick Auto").clicked() {
                    self.run_action(AppAction::QuickAuto);
                }
                if ui.button("Undo").clicked() {
                    self.run_action(AppAction::Undo);
                }
                if ui.button("Redo").clicked() {
                    self.run_action(AppAction::Redo);
                }
            });
        } else {
            ui.label("行を選択してください。");
        }
    }

    fn draw_log_tab(&mut self, ui: &mut egui::Ui) {
        if !self.state.file_status.is_empty() {
            ui.label(&self.state.file_status);
        }
        if !self.state.dict_status.is_empty() {
            ui.label(&self.state.dict_status);
        }
        if !self.state.dict_prefs_error.is_empty() {
            ui.colored_label(egui::Color32::RED, &self.state.dict_prefs_error);
        }
        if let Some(summary) = &self.state.dict_build_summary {
            ui.label(format!(
                "辞書情報: built_at(unix)={} pairs={} files={} pair_files={}",
                summary.built_at_unix, summary.pairs, summary.files_seen, summary.file_pairs
            ));
        }
        if let Some(err) = &self.state.xml_error {
            ui.colored_label(egui::Color32::RED, err);
        }
        if let Some(err) = &self.state.hybrid_error {
            ui.colored_label(egui::Color32::RED, err);
        }
        if let Some(status) = &self.state.diff_status {
            ui.label(format!("Diff status: {status:?}"));
        }
        if !self.state.encoding_status.is_empty() {
            ui.label(&self.state.encoding_status);
        }
        for issue in &self.state.validation_issues {
            ui.label(format!("{}: {}", issue.rule_id, issue.message));
        }
    }

    fn draw_aux_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Dictionary");

        let mut src = self.state.dict_source_lang.clone();
        if ui.text_edit_singleline(&mut src).changed() {
            self.run_action(AppAction::SetDictSourceLang(src));
        }

        let mut dst = self.state.dict_target_lang.clone();
        if ui.text_edit_singleline(&mut dst).changed() {
            self.run_action(AppAction::SetDictTargetLang(dst));
        }

        let mut root = self.state.dict_root.clone();
        if ui.text_edit_singleline(&mut root).changed() {
            self.run_action(AppAction::SetDictRoot(root));
        }

        ui.horizontal(|ui| {
            if ui.button("辞書を構築").clicked() {
                self.run_action(AppAction::BuildDictionary);
            }
            if ui.button("言語ペア初期化").clicked() {
                self.run_action(AppAction::ResetDictLanguagePair);
            }
        });

        ui.separator();
        ui.label("XML");
        let mut xml = self.state.xml_text.clone();
        if ui
            .add(
                TextEdit::multiline(&mut xml)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.run_action(AppAction::SetXmlText(xml));
        }
        ui.horizontal(|ui| {
            if ui.button("XML適用").clicked() {
                self.run_action(AppAction::ApplyXmlFromEditor);
            }
            if ui.button("XML書き出し").clicked() {
                self.run_action(AppAction::ExportXmlToEditor);
            }
        });
    }
}

impl eframe::App for XtransApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
            self.run_action(AppAction::QuickAuto);
        }

        TopBottomPanel::top("menu_toolbar").show(ctx, |ui| {
            self.draw_menu(ui);
            ui.separator();
            self.draw_toolbar(ui);
        });

        TopBottomPanel::bottom("status").show(ctx, |ui| {
            let counts = self.state.channel_counts();
            let ratio = self.state.translation_ratio() / 100.0;
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::ProgressBar::new(ratio).desired_width(180.0));
                ui.label(format!(
                    "[{}] -> [{}]",
                    self.state.dict_source_lang, self.state.dict_target_lang
                ));
                ui.label(RichText::new(&self.state.file_status).small());
                ui.label(format!("{}/{}", counts.translated, counts.total));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.42);
                    self.draw_entry_list(ui);
                });
                ui.separator();
                ui.vertical(|ui| {
                    self.draw_tabs(ui);
                    ui.separator();
                    if self.state.active_tab == Tab::Home {
                        self.draw_home_tab(ui);
                    } else if self.state.active_tab == Tab::Log {
                        self.draw_log_tab(ui);
                    } else {
                        ui.label("このタブは次フェーズで実装します。");
                    }
                    self.draw_aux_panel(ui);
                });
            });
        });
    }
}
