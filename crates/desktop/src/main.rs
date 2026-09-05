//! Native desktop application runner for the Serverless & Desktop Template.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;


use app::TemplateApp;
use eframe::egui;
use eframe::NativeOptions;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> eframe::Result<()> {
    // Logging setup: INFO for application logs, WARN for external library modules
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,wgpu=warn,egui=warn")),
        )
        .init();

    // Native window viewport configurations
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Serverless & Desktop Template")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([700.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Serverless & Desktop Template",
        options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    )
}

