mod app;
mod cli;
mod qt_shell;

use app::EditorApp;
use cli::CliOptions;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = CliOptions::parse(std::env::args())?;
    let app = EditorApp::open_path(&options.scene_path).map_err(|error| error.to_string())?;

    if options.dump_view_model {
        let json = serde_json::to_string_pretty(&app.view_model())
            .map_err(|error| format!("failed to serialize view model: {error}"))?;
        println!("{json}");
        return Ok(());
    }

    if options.qt_shell_requested {
        println!("{}", qt_shell::launch_unavailable_message());
    }

    let summary = app.summary();
    println!("tweaky editor");
    println!("path: {}", summary.document_path.display());
    println!(
        "document: {} ({}x{})",
        summary.document_name, summary.canvas_width, summary.canvas_height
    );
    println!("render items: {}", summary.render_item_count);

    if let Some(selected) = summary.selected {
        println!(
            "selected: {} [{}] {}",
            selected.id, selected.node_type, selected.name
        );
    }

    println!("hierarchy:");
    for entry in &app.state.hierarchy {
        println!(
            "{}- {} [{}] {}",
            "  ".repeat(entry.depth),
            entry.node_id,
            entry.node_type,
            entry.name
        );
    }

    if let Some(path) = options.export_path {
        app.export_png(&path).map_err(|error| error.to_string())?;
        println!("exported: {}", path.display());
    }

    Ok(())
}
