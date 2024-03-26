use eframe::epaint::Shape::Path;
use egui::{CentralPanel, Color32, Frame, pos2, Shape, Stroke, Ui};
use egui::epaint::PathShape;
use serde::de::Unexpected::Str;

pub(crate) fn draw_yin_yang(ui: &mut Ui, radius: f32) {
    let background_color = Color32::from_white_alpha(200);
    let yin_color = Color32::from_black_alpha(255);
    let yang_color = Color32::from_white_alpha(255);
    let stroke = Stroke::new(10.0, background_color);

    // Draw the background circle
    let frame = Frame::none();
    let pos = ui.min_size().to_pos2();
    let (x,y) = (pos.x / 2., pos.y / 2.);
    let center= pos2(x,y);


    ui.painter().circle(center, radius, background_color,  stroke);
    let stroke = Stroke::new(10.0, yin_color);
    // // Draw the yin-yang's components
    ui.painter()
        .circle(center, radius / 2., yin_color, stroke);
    let stroke = Stroke::new(10.0, yang_color);
    ui.painter()
        .circle(center,radius / 2., yang_color, stroke);

    // Draw the two smaller circles inside the yin and yang part
    ui.painter()
        .circle(center + egui::Vec2::new(radius / 2.0, 0.0), radius/4., background_color, stroke);
    let stroke = Stroke::new(10.0, yin_color);
    ui.painter()
        .circle(center + egui::Vec2::new(radius / 2.0, 0.0), radius/2.,  background_color, stroke);

    // Draw the curve dividing the yin and yang parts
    let shape = PathShape {stroke, fill: yin_color, closed: true, points: Vec::new()};


    // Shape::Path()
    //     .arc(
    //     center,
    //     radius,
    //     0.0,
    //     std::f32::consts::PI,
    //     egui::Stroke::new(1.0, yin_color),
    // );


    ui.painter().add(Shape::Path(shape));

    // let mut path = Path::new();
    // path.arc(
    //     center,
    //     radius,
    //     0.0,
    //     -std::f32::consts::PI,
    //     egui::Stroke::new(1.0, yang_color),
    // );
    //ui.painter().add(egui::Shape::path(shape));
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