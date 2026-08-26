use std::collections::VecDeque;

use walkers::{Map, MapMemory, lat_lon};
use walkers::sources::{Attribution, TileSource};
use walkers::TileId;

pub struct EsriSatellite;

impl TileSource for EsriSatellite {
    fn tile_url(&self, tile_id: TileId) -> String {
        format!(
            "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{}/{}/{}",
            tile_id.zoom, tile_id.y, tile_id.x
        )
    }

    fn attribution(&self) -> Attribution {
        Attribution {
            text: "Esri, Maxar, Earthstar Geographics",
            url: "https://www.esri.com/",
            logo_light: None,
            logo_dark: None,
        }
    }

    fn max_zoom(&self) -> u8 {
        18
    }
}

pub struct MapState {
    pub tiles: walkers::HttpTiles,
    pub memory: MapMemory,
}

impl MapState {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            tiles: walkers::HttpTiles::new(EsriSatellite, ctx.clone()),
            memory: MapMemory::default(),
        }
    }
}

pub fn gps_map(
    ui: &mut egui::Ui,
    _trail: &VecDeque<(f64, f64)>,
    current_pos: Option<(f64, f64)>,
    ground_pos: Option<(f64, f64)>,
    map_state: &mut MapState,
) -> egui::Rect {
    let my_pos = current_pos
        .map(|(lat, lon)| lat_lon(lat, lon))
        .unwrap_or_else(|| {
            ground_pos
                .map(|(lat, lon)| lat_lon(lat, lon))
                .unwrap_or_else(|| lat_lon(0.0, 0.0))
        });

    let map_height = ui.available_height().max(200.0);
    let map_width = ui.available_width();

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(map_width, map_height),
        egui::Sense::hover(),
    );

    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect),
    );

    Map::new(
        Some(&mut map_state.tiles),
        &mut map_state.memory,
        my_pos,
    )
    .show(&mut child_ui, |ui, _response, projector, _memory| {
        let painter = ui.painter();
        let to_pos = |v: egui::Vec2| egui::pos2(v.x, v.y);

        if let Some((lat, lon)) = ground_pos {
            let pos = to_pos(projector.project(lat_lon(lat, lon)));
            painter.circle_filled(pos, 7.0, egui::Color32::BLACK);
            painter.circle_filled(pos, 5.0, egui::Color32::from_rgb(0, 220, 255));
        }

        if let Some((lat, lon)) = current_pos {
            let rocket = to_pos(projector.project(lat_lon(lat, lon)));

            if let Some((glat, glon)) = ground_pos {
                let ground = to_pos(projector.project(lat_lon(glat, glon)));
                painter.line_segment(
                    [ground, rocket],
                    egui::Stroke::new(4.0, egui::Color32::BLACK),
                );
                painter.line_segment(
                    [ground, rocket],
                    egui::Stroke::new(2.0, egui::Color32::RED),
                );
            }

            painter.circle_filled(rocket, 8.0, egui::Color32::BLACK);
            painter.circle_filled(rocket, 6.0, egui::Color32::RED);
        }
    });
    rect
}
