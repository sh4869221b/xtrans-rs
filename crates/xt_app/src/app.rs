use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, FontData, FontDefinitions, FontFamily, Layout, RichText,
    ScrollArea, TextEdit, TopBottomPanel,
};
use xt_core::dictionary::{DictionaryBuildStats, TranslationDictionary};
use xt_core::import_export::{apply_xml_default, import_entries, XmlApplyStats};
use xt_core::model::Entry;

use crate::actions::{
    apply_quick_auto_selection, dispatch, run_save_job, AppAction, SaveJobData, SaveMode,
};
use crate::state::{row_fields, AppState, Tab};

const LARGE_XML_EDITOR_THRESHOLD_BYTES: usize = 256 * 1024;
const ENTRY_COL_EDID_WIDTH: f32 = 120.0;
const ENTRY_COL_RECORD_WIDTH: f32 = 84.0;
const ENTRY_COL_TEXT_WIDTH: f32 = 240.0;
const ENTRY_COL_LD_WIDTH: f32 = 26.0;
const XT_ACCENT: Color32 = Color32::from_rgb(42, 157, 194);

pub fn launch() -> eframe::Result<()> {
    crate::hotpatch::init_hotpatch();
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
    fonts_configured: bool,
    style_configured: bool,
    pending_job: Option<PendingJob>,
    show_large_xml_editor: bool,
}

struct PendingJob {
    started_at: Instant,
    label: String,
    receiver: Receiver<JobResult>,
}

enum JobResult {
    Xml(Result<XmlApplyResult, String>),
    BuildDictionary(Result<BuildDictionaryResult, String>),
    QuickAuto(Result<QuickAutoResult, String>),
    Save(Result<SaveResult, String>),
}

struct XmlApplyResult {
    source_label: Option<String>,
    xml_text: String,
    merged: Vec<Entry>,
    stats: XmlApplyStats,
}

struct BuildDictionaryResult {
    dict: TranslationDictionary,
    stats: DictionaryBuildStats,
}

struct QuickAutoResult {
    next: Vec<Entry>,
    updated: usize,
}

struct SaveResult {
    path: PathBuf,
    mode: SaveMode,
}

impl XtransApp {
    fn run_action(&mut self, action: AppAction) {
        if let Err(err) = dispatch(&mut self.state, action) {
            if self.state.file_status.is_empty() {
                self.state.file_status = err;
            }
        }
    }

    fn is_blocked(&self) -> bool {
        self.pending_job.is_some()
    }

    fn try_start_job<F>(&mut self, label: impl Into<String>, spawn: F) -> bool
    where
        F: FnOnce(Sender<JobResult>) + Send + 'static,
    {
        if self.pending_job.is_some() {
            self.state.file_status = "重い処理を実行中です".to_string();
            return false;
        }
        let label = label.into();
        let (tx, rx) = mpsc::channel::<JobResult>();
        thread::spawn(move || spawn(tx));
        self.pending_job = Some(PendingJob {
            started_at: Instant::now(),
            label: label.clone(),
            receiver: rx,
        });
        self.state.file_status = format!("{label}...");
        true
    }

    fn start_xml_apply(&mut self, contents: String, source_label: Option<String>) {
        let current_entries = self.state.entries().to_vec();
        let source_label_for_job = source_label.clone();
        if !self.try_start_job("XML適用", move |tx| {
            let result = import_entries(&contents)
                .map_err(|err| format!("{err:?}"))
                .map(|imported| {
                    let (merged, stats) = apply_xml_default(&current_entries, &imported);
                    XmlApplyResult {
                        source_label: source_label_for_job,
                        xml_text: contents,
                        merged,
                        stats,
                    }
                });
            let _ = tx.send(JobResult::Xml(result));
        }) {
            return;
        }
        self.state.xml_error = None;
    }

    fn start_build_dictionary_job(&mut self) {
        let root = self.state.dict_root.clone();
        let source_lang = self.state.dict_source_lang.clone();
        let target_lang = self.state.dict_target_lang.clone();
        if !self.try_start_job("辞書構築", move |tx| {
            let result = TranslationDictionary::build_from_strings_dir(
                &PathBuf::from(root),
                &source_lang,
                &target_lang,
            )
            .map_err(|err| format!("辞書構築失敗: {err}"))
            .map(|(dict, stats)| BuildDictionaryResult { dict, stats });
            let _ = tx.send(JobResult::BuildDictionary(result));
        }) {
            return;
        }
        self.state.dict_status = "辞書構築中...".to_string();
    }

    fn start_quick_auto_job(&mut self) {
        let dict = self.state.dict.clone();
        let entries = self.state.entries().to_vec();
        let selected = self.state.selected_key();
        if !self.try_start_job("Quick自動翻訳", move |tx| {
            let result = apply_quick_auto_selection(dict.as_ref(), &entries, selected)
                .map_err(|err| err.to_string())
                .map(|(next, updated)| QuickAutoResult { next, updated });
            let _ = tx.send(JobResult::QuickAuto(result));
        }) {
            return;
        }
        self.state.dict_status = "Quick自動翻訳中...".to_string();
    }

    fn start_save_job(&mut self, mode: SaveMode) {
        let data = SaveJobData::from_state(&self.state);
        let label = match &mode {
            SaveMode::Overwrite => "保存",
            SaveMode::Auto | SaveMode::Path(_) => "別名保存",
        };
        let mode_for_job = mode.clone();
        let _ = self.try_start_job(label, move |tx| {
            let result = run_save_job(data, mode_for_job.clone())
                .map(|path| SaveResult {
                    path,
                    mode: mode_for_job,
                })
                .map_err(|err| format!("保存失敗: {err}"));
            let _ = tx.send(JobResult::Save(result));
        });
    }

    fn poll_job(&mut self) {
        let Some(pending) = self.pending_job.as_mut() else {
            return;
        };

        match pending.receiver.try_recv() {
            Ok(job_result) => {
                let elapsed = pending.started_at.elapsed();
                self.pending_job = None;
                match job_result {
                    JobResult::Xml(Ok(done)) => {
                        let xml_len = done.xml_text.len();
                        let source_label = done.source_label;
                        let drop_large_xml_text =
                            source_label.is_some() && xml_len > LARGE_XML_EDITOR_THRESHOLD_BYTES;
                        if drop_large_xml_text {
                            self.state.xml_text.clear();
                        } else {
                            self.state.xml_text = done.xml_text;
                        }
                        if done.stats.updated > 0 {
                            self.state.apply_target_updates_with_history(done.merged);
                        }
                        self.state.last_xml_stats = Some(done.stats);
                        self.state.xml_error = None;
                        self.show_large_xml_editor =
                            !drop_large_xml_text && xml_len <= LARGE_XML_EDITOR_THRESHOLD_BYTES;
                        let src = source_label.unwrap_or_else(|| "エディタ".to_string());
                        let mut status = format!(
                            "XML適用({src}): updated={} unchanged={} missing={} [{:.2}s]",
                            self.state
                                .last_xml_stats
                                .as_ref()
                                .map(|s| s.updated)
                                .unwrap_or(0),
                            self.state
                                .last_xml_stats
                                .as_ref()
                                .map(|s| s.unchanged)
                                .unwrap_or(0),
                            self.state
                                .last_xml_stats
                                .as_ref()
                                .map(|s| s.missing)
                                .unwrap_or(0),
                            elapsed.as_secs_f32()
                        );
                        if drop_large_xml_text {
                            status.push_str(" [XML本文は保持しません]");
                        }
                        self.state.file_status = status;
                    }
                    JobResult::Xml(Err(err)) => {
                        self.state.xml_error = Some(err.clone());
                        self.state.file_status =
                            format!("XML適用失敗 [{:.2}s]", elapsed.as_secs_f32());
                    }
                    JobResult::BuildDictionary(Ok(done)) => {
                        let pairs = done.dict.len();
                        self.state.dict = Some(done.dict);
                        self.state.mark_dictionary_built(
                            pairs,
                            done.stats.files_seen,
                            done.stats.file_pairs,
                        );
                        self.state.dict_status = format!(
                            "辞書構築: pairs={} files={} pair_files={}",
                            pairs, done.stats.files_seen, done.stats.file_pairs
                        );
                        self.state.file_status =
                            format!("辞書構築完了 [{:.2}s]", elapsed.as_secs_f32());
                    }
                    JobResult::BuildDictionary(Err(err)) => {
                        self.state.dict_status = err.clone();
                        self.state.file_status =
                            format!("辞書構築失敗 [{:.2}s]", elapsed.as_secs_f32());
                    }
                    JobResult::QuickAuto(Ok(done)) => {
                        if done.updated > 0 {
                            self.state.apply_target_updates_with_history(done.next);
                        }
                        self.state.dict_status = format!("Quick自動翻訳: updated={}", done.updated);
                        self.state.file_status =
                            format!("Quick自動翻訳完了 [{:.2}s]", elapsed.as_secs_f32());
                    }
                    JobResult::QuickAuto(Err(err)) => {
                        self.state.dict_status = err.clone();
                        self.state.file_status =
                            format!("Quick自動翻訳失敗 [{:.2}s]", elapsed.as_secs_f32());
                    }
                    JobResult::Save(Ok(done)) => {
                        let prefix = match done.mode {
                            SaveMode::Overwrite => "保存",
                            SaveMode::Auto | SaveMode::Path(_) => "別名保存",
                        };
                        self.state.file_status = format!(
                            "{}: {} [{:.2}s]",
                            prefix,
                            done.path.display(),
                            elapsed.as_secs_f32()
                        );
                    }
                    JobResult::Save(Err(err)) => {
                        self.state.file_status = format!("{err} [{:.2}s]", elapsed.as_secs_f32());
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.pending_job = None;
                self.state.file_status = "重い処理ワーカーが異常終了しました".to_string();
            }
        }
    }

    fn draw_busy_overlay(&self, ctx: &egui::Context) {
        let Some(pending) = self.pending_job.as_ref() else {
            return;
        };
        let rect = ctx.screen_rect();
        let layer =
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("xml_apply_backdrop"));
        let painter = ctx.layer_painter(layer);
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(180));

        egui::Area::new(egui::Id::new("xml_apply_modal"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label(format!("{}を実行しています", pending.label));
                        ui.label(format!(
                            "経過: {:.1}s",
                            pending.started_at.elapsed().as_secs_f32()
                        ));
                        ui.label("完了まで操作はできません");
                    });
                });
            });
    }

    fn draw_entry_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [ENTRY_COL_EDID_WIDTH, 18.0],
                egui::Label::new(RichText::new("EDID").color(XT_ACCENT).small().monospace()),
            );
            ui.add_sized(
                [ENTRY_COL_RECORD_WIDTH, 18.0],
                egui::Label::new(RichText::new("Record").color(XT_ACCENT).small()),
            );
            ui.add_sized(
                [ENTRY_COL_TEXT_WIDTH, 18.0],
                egui::Label::new(RichText::new("Source").color(XT_ACCENT).small()),
            );
            ui.add_sized(
                [ENTRY_COL_TEXT_WIDTH, 18.0],
                egui::Label::new(RichText::new("Target").color(XT_ACCENT).small()),
            );
            ui.add_sized(
                [ENTRY_COL_LD_WIDTH, 18.0],
                egui::Label::new(RichText::new("LD").color(XT_ACCENT).small().monospace()),
            );
        });
        ui.separator();
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
                        match std::fs::read_to_string(&path) {
                            Ok(contents) => {
                                self.start_xml_apply(contents, Some(path.display().to_string()))
                            }
                            Err(err) => {
                                self.state.file_status = format!("read {}: {err}", path.display());
                            }
                        }
                    }
                }
                if ui.button("翻訳XMLを書き出し").clicked() {
                    ui.close_menu();
                    self.run_action(AppAction::ExportXmlToEditor);
                }
                if ui.button("上書き保存").clicked() {
                    ui.close_menu();
                    self.start_save_job(SaveMode::Overwrite);
                }
                if ui.button("別名保存").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        self.start_save_job(SaveMode::Path(path));
                    } else {
                        self.start_save_job(SaveMode::Auto);
                    }
                }
            });

            ui.menu_button("翻訳", |ui| {
                if ui.button("辞書を構築").clicked() {
                    ui.close_menu();
                    self.start_build_dictionary_job();
                }
                if ui.button("Quick自動翻訳 (Ctrl-R)").clicked() {
                    ui.close_menu();
                    self.start_quick_auto_job();
                }
            });

            ui.menu_button("オプション", |ui| {
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
        });

        let counts = self.state.channel_counts();
        let ratio = if counts.total == 0 {
            0.0
        } else {
            counts.translated as f32 / counts.total as f32
        };
        ui.horizontal(|ui| {
            ui.label(format!(
                "STRINGS [{}/{}]",
                counts.translated, counts.strings
            ));
            ui.add(egui::ProgressBar::new(ratio).desired_width(140.0));
        });
    }

    fn draw_entry_list(&mut self, ui: &mut egui::Ui) {
        let filtered_len = self.state.filtered_len();
        let selected_key = self.state.selected_key();
        let mut next_selection = None;
        ui.label(RichText::new("Entries").color(XT_ACCENT).strong());
        ui.separator();
        self.draw_entry_header(ui);

        ScrollArea::vertical().show_rows(ui, 22.0, filtered_len, |ui, row_range| {
            for row in row_range {
                let Some(entry) = self.state.filtered_entry(row) else {
                    continue;
                };
                let selected = selected_key.as_deref() == Some(entry.key.as_str());
                let (edid, record_id, ld) = row_fields(&entry.key, &entry.target_text);
                ui.horizontal(|ui| {
                    let source_preview = text_preview(&entry.source_text, 72);
                    let target_preview = text_preview(&entry.target_text, 72);
                    let clicked = ui
                        .add_sized(
                            [ENTRY_COL_EDID_WIDTH, 18.0],
                            egui::SelectableLabel::new(
                                selected,
                                RichText::new(edid).monospace().size(12.0),
                            ),
                        )
                        .clicked()
                        || ui
                            .add_sized(
                                [ENTRY_COL_RECORD_WIDTH, 18.0],
                                egui::Label::new(record_id).sense(egui::Sense::click()),
                            )
                            .clicked()
                        || ui
                            .add_sized(
                                [ENTRY_COL_TEXT_WIDTH, 18.0],
                                egui::Label::new(source_preview).sense(egui::Sense::click()),
                            )
                            .clicked()
                        || ui
                            .add_sized(
                                [ENTRY_COL_TEXT_WIDTH, 18.0],
                                egui::Label::new(target_preview).sense(egui::Sense::click()),
                            )
                            .clicked()
                        || ui
                            .add_sized(
                                [ENTRY_COL_LD_WIDTH, 18.0],
                                egui::Label::new(RichText::new(ld).monospace())
                                    .sense(egui::Sense::click()),
                            )
                            .clicked();
                    if clicked {
                        next_selection = Some(entry.key.clone());
                    }
                });
                ui.separator();
            }
        });

        if let Some(key) = next_selection {
            self.run_action(AppAction::SelectEntry(key));
        }
    }

    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in Tab::all() {
                let selected = self.state.active_tab == tab;
                let text = if selected {
                    RichText::new(label).color(XT_ACCENT).strong()
                } else {
                    RichText::new(label)
                };
                if ui
                    .add_sized([110.0, 22.0], egui::SelectableLabel::new(selected, text))
                    .clicked()
                {
                    self.run_action(AppAction::SetActiveTab(tab));
                }
            }
        });
    }

    fn draw_home_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(key) = self.state.selected_key() {
            ui.label(RichText::new(format!("Key: {key}")).color(XT_ACCENT));
            ui.add(
                TextEdit::multiline(&mut self.state.edit_source)
                    .desired_rows(4)
                    .hint_text("原文"),
            );
            ui.add(
                TextEdit::multiline(&mut self.state.edit_target)
                    .desired_rows(4)
                    .hint_text("訳文"),
            );

            ui.horizontal(|ui| {
                if ui.button("Apply Edit").clicked() {
                    self.run_action(AppAction::ApplyEdit);
                }
                if ui.button("Quick Auto").clicked() {
                    self.start_quick_auto_job();
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
        ui.label(RichText::new("Dictionary").color(XT_ACCENT).strong());

        if ui
            .text_edit_singleline(&mut self.state.dict_source_lang)
            .changed()
        {
            self.state.persist_dictionary_prefs();
        }

        if ui
            .text_edit_singleline(&mut self.state.dict_target_lang)
            .changed()
        {
            self.state.persist_dictionary_prefs();
        }

        if ui.text_edit_singleline(&mut self.state.dict_root).changed() {
            self.state.persist_dictionary_prefs();
        }

        ui.horizontal(|ui| {
            if ui.button("辞書を構築").clicked() {
                self.start_build_dictionary_job();
            }
            if ui.button("言語ペア初期化").clicked() {
                self.run_action(AppAction::ResetDictLanguagePair);
            }
        });

        ui.separator();
        ui.label(RichText::new("XML").color(XT_ACCENT).strong());
        let xml_len = self.state.xml_text.len();
        let suppress_large_editor =
            xml_len > LARGE_XML_EDITOR_THRESHOLD_BYTES && !self.show_large_xml_editor;
        if suppress_large_editor {
            ui.label(format!(
                "XMLエディタを省略中: {} KB (閾値 {} KB)",
                xml_len / 1024,
                LARGE_XML_EDITOR_THRESHOLD_BYTES / 1024
            ));
            ui.horizontal(|ui| {
                if ui.button("XMLエディタを開く（重い）").clicked() {
                    self.show_large_xml_editor = true;
                }
                if ui.button("XMLテキストをクリア").clicked() {
                    self.state.xml_text.clear();
                    self.show_large_xml_editor = false;
                }
            });
        } else {
            ui.add(
                TextEdit::multiline(&mut self.state.xml_text)
                    .code_editor()
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            if xml_len > LARGE_XML_EDITOR_THRESHOLD_BYTES {
                if ui.button("XMLエディタを閉じる（軽量表示へ）").clicked() {
                    self.show_large_xml_editor = false;
                }
            }
        }
        ui.horizontal(|ui| {
            if ui.button("XML適用").clicked() {
                self.start_xml_apply(self.state.xml_text.clone(), None);
            }
            if ui.button("XML書き出し").clicked() {
                self.run_action(AppAction::ExportXmlToEditor);
            }
        });
    }

    fn update_inner(&mut self, ctx: &egui::Context) {
        if !self.fonts_configured {
            configure_japanese_font(ctx);
            self.fonts_configured = true;
        }
        if !self.style_configured {
            configure_xtranslator_style(ctx);
            self.style_configured = true;
        }
        self.poll_job();
        let blocked = self.is_blocked();
        if blocked {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        if !blocked && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
            self.start_quick_auto_job();
        }

        TopBottomPanel::top("menu_toolbar").show(ctx, |ui| {
            ui.add_enabled_ui(!blocked, |ui| {
                self.draw_menu(ui);
                ui.separator();
                self.draw_toolbar(ui);
            });
        });

        TopBottomPanel::bottom("status").show(ctx, |ui| {
            let counts = self.state.channel_counts();
            let ratio = if counts.total == 0 {
                0.0
            } else {
                counts.translated as f32 / counts.total as f32
            };
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
            ui.add_enabled_ui(!blocked, |ui| {
                ui.vertical(|ui| {
                    let list_height = (ui.available_height() * 0.46).max(200.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), list_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            self.draw_entry_list(ui);
                        },
                    );

                    ui.separator();
                    self.draw_tabs(ui);
                    ui.separator();
                    if self.state.active_tab == Tab::Home {
                        self.draw_home_tab(ui);
                    } else if self.state.active_tab == Tab::Log {
                        self.draw_log_tab(ui);
                    } else {
                        self.state.active_tab = Tab::Home;
                        self.draw_home_tab(ui);
                    }
                    self.draw_aux_panel(ui);
                });
            });
        });

        if blocked {
            self.draw_busy_overlay(ctx);
        }
    }
}

impl eframe::App for XtransApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(all(debug_assertions, feature = "hotpatch"))]
        {
            subsecond::call(|| self.update_inner(ctx));
        }
        #[cfg(not(all(debug_assertions, feature = "hotpatch")))]
        {
            self.update_inner(ctx);
        }
    }
}

fn configure_japanese_font(ctx: &egui::Context) {
    let Some(bytes) = load_japanese_font_bytes() else {
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("xtrans-jp".to_string(), FontData::from_owned(bytes).into());

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "xtrans-jp".to_string());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, "xtrans-jp".to_string());
    }

    ctx.set_fonts(fonts);
}

fn configure_xtranslator_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();

    style.visuals.override_text_color = Some(Color32::from_rgb(212, 218, 224));
    style.visuals.panel_fill = Color32::from_rgb(20, 22, 26);
    style.visuals.window_fill = Color32::from_rgb(22, 24, 28);
    style.visuals.faint_bg_color = Color32::from_rgb(28, 31, 36);
    style.visuals.extreme_bg_color = Color32::from_rgb(9, 10, 13);
    style.visuals.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(58, 63, 71));
    style.visuals.window_rounding = egui::Rounding::same(3.0);
    style.visuals.menu_rounding = egui::Rounding::same(2.0);

    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(42, 157, 194, 120);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, XT_ACCENT);
    style.visuals.hyperlink_color = XT_ACCENT;

    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 26, 31);
    style.visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(21, 23, 27);
    style.visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(46, 50, 57));
    style.visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(150, 158, 168));

    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(33, 36, 41);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(28, 31, 36);
    style.visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(70, 76, 86));
    style.visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(196, 203, 211));

    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 45, 52);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(36, 41, 47);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, XT_ACCENT);
    style.visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.5, Color32::from_rgb(225, 231, 238));

    style.visuals.widgets.active.bg_fill = Color32::from_rgb(30, 58, 71);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(28, 51, 61);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, XT_ACCENT);
    style.visuals.widgets.active.fg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(230, 236, 242));

    style.visuals.widgets.open.bg_fill = Color32::from_rgb(32, 37, 43);
    style.visuals.widgets.open.weak_bg_fill = Color32::from_rgb(29, 34, 40);
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, XT_ACCENT);
    style.visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(215, 222, 230));

    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.indent = 16.0;
    style.spacing.slider_width = 140.0;
    style.spacing.text_edit_width = 220.0;

    ctx.set_style(style);
}

fn load_japanese_font_bytes() -> Option<Vec<u8>> {
    if let Ok(path) = std::env::var("XTRANS_FONT") {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }

    let candidates = [
        "/usr/share/fonts/OTF/ipagp.ttf",
        "/usr/share/fonts/OTF/ipag.ttf",
        "/usr/share/fonts/OTF/ipamp.ttf",
        "/usr/share/fonts/OTF/ipam.ttf",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Medium.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Light.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansJP-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.ttf",
        "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf",
        "/usr/share/fonts/truetype/ipafont-gothic/ipag.ttf",
        "/usr/share/fonts/opentype/vlgothic/VL-Gothic-Regular.ttf",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            if let Ok(bytes) = std::fs::read(candidate) {
                return Some(bytes);
            }
        }
    }

    None
}

fn text_preview(text: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}
