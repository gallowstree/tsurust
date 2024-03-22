use eframe::epaint::Shape::Path;
use egui::{CentralPanel, Color32, Frame, Ui};

pub(crate) fn draw_yin_yang(ui: &mut Ui, radius: f32) {
    let background_color = Color32::from_white_alpha(255);
    let yin_color = Color32::from_black_alpha(255);
    let yang_color = Color32::from_white_alpha(255);

    // Draw the background circle
    let frame = Frame::none();
    let center= ui.min_size().to_pos2();
    //ui.painter().circle(center, radius, background_color, Default::default());

    // // Draw the yin-yang's components
    // ui.painter()
    //     .circle(center, center - egui::Vec2::new(radius / 2.0, 0.0), radius / 2.0, yin_color);
    // ui.painter()
    //     .circle(center, center + egui::Vec2::new(radius / 2.0, 0.0), radius / 2.0, yang_color);

    // Draw the two smaller circles inside the yin and yang parts
    // ui.painter()
    //     .circle(frame, ui.center() - egui::Vec2::new(radius / 2.0, 0.0), radius / 4.0, background_color);
    // ui.painter()
    //     .circle(frame, ui.center() + egui::Vec2::new(radius / 2.0, 0.0), radius / 4.0, background_color);

    // Draw the curve dividing the yin and yang parts
    // let mut path = Path::new();
    // path.arc(
    //     ui.center(),
    //     radius,
    //     0.0,
    //     std::f32::consts::PI,
    //     egui::Stroke::new(1.0, yin_color),
    // );
    // ui.painter().add(egui::Shape::path(path));
    //
    // let mut path = Path::new();
    // path.arc(
    //     ui.center(),
    //     radius,
    //     0.0,
    //     -std::f32::consts::PI,
    //     egui::Stroke::new(1.0, yang_color),
    // );
    // ui.painter().add(egui::Shape::path(path));
}

fn main() {
    // let mut app = eframe::NativeOptions::default();
    // let (width, height) = (400, 400);
    //
    // let native_options = eframe::NativeOptions::default();
    //
    // eframe::run_native(
    //     "eframe template",
    //     native_options,
    //     Box::new(|_window, _egui_ctx, _frame| {
    //         CentralPanel::default().show(_egui_ctx, |ui| {
    //             // Draw yin-yang symbol
    //             draw_yin_yang(ui, width as f32 / 4.0);
    //         });
    //     }
    // );
}