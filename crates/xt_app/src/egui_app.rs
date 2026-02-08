use eframe::egui;

pub fn run() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "xtrans",
        options,
        Box::new(|_cc| Box::<EguiApp>::default()),
    )
    .expect("failed to launch egui application");
}

#[derive(Default)]
struct EguiApp;

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("xtrans");
            ui.label("egui/eframe mode is the default UI entry point.");
        });
    }
}
