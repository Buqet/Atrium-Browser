
use eframe::egui;

struct TestApp;

impl eframe::App for TestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello World!");
        });
    }
}

fn main() -> eframe::Result<()> {
    eprintln!("Starting minimal egui test...");
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Test",
        options,
        Box::new(|_cc| Ok(Box::new(TestApp))),
    );
    
    eprintln!("Result: {:?}", result);
    result
}
