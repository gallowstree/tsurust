use egui::{ScrollArea, Widget};
use tsurust_common::board::*;
use crate::tile_button::TileButton;


pub struct HandRenderer<'a> {
    tiles: &'a mut Vec<Tile>,
}

impl<'a> HandRenderer<'a> {
    pub fn new(tiles: &'a mut Vec<Tile>) -> Self {
        Self { tiles }
    }
}

impl<'a> Widget for HandRenderer<'a> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical_centered(|ui| {
            ScrollArea::vertical()
                .show(ui, |ui| {
                    for tile in self.tiles {
                        ui.add_space(10.);
                        let button = TileButton::new(tile);
                        ui.add(button);
                    }
                });
        }).response

    }
}