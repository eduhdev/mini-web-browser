use eframe::egui;

use crate::network::{lex, Url};

pub fn run(url: Option<String>) -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Browser",
        options,
        Box::new(|_cc| Ok(Box::new(Browser::new(url)))),
    )
}

struct Browser {
    text: String,
}

impl Browser {
    fn new(url: Option<String>) -> Self {
        let text = match url {
            Some(url) => {
                let body = Url::new(&url).request();
                lex(&body)
            }
            None => String::new(),
        };

        Self { text }
    }

    fn load(&self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let font_id = egui::FontId::proportional(16.0);
        let color = ui.visuals().text_color();

        for c in self.text.chars() {
            painter.text(
                egui::pos2(100.0, 100.0),
                egui::Align2::LEFT_TOP,
                c,
                font_id.clone(),
                color,
            );
        }
    }
}

impl eframe::App for Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.load(ui);
        });
    }
}
