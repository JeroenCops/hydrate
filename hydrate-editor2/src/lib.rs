mod app;
mod ui;

mod db_state;
pub use db_state::DbState;

mod persistent_app_state;

mod action_queue;

mod egui_debug_ui;
mod modal_action;
mod ui_state;

use crate::app::HydrateEditorApp;
use hydrate_model::pipeline::AssetEngine;

pub fn run(
    db_state: DbState,
    asset_engine: AssetEngine,
) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([450.0, 300.0]),
        //.with_min_inner_size([900.0, 650.0]),
        follow_system_theme: false,
        default_theme: eframe::Theme::Dark,
        centered: true,
        window_builder: Some(Box::new(|mut builder| {
            builder.position = Some(egui::pos2(1000.0, 0.0));
            builder.inner_size = Some(egui::vec2(700.0, 500.0));
            builder
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Hydrate Editor",
        native_options,
        Box::new(|cc| Box::new(HydrateEditorApp::new(cc, db_state, asset_engine))),
    )
}
