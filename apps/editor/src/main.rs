mod app;
mod cli;
mod qt_shell;

use app::EditorApp;
use cli::{CliOptions, NodeJsonEdit, NodePositionEdit, RenameNodeOptions};

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = CliOptions::parse(std::env::args())?;
    let has_edits = options.rename_node.is_some()
        || options.set_position.is_some()
        || options.set_params_json.is_some()
        || options.set_style_json.is_some();
    let mut app = EditorApp::open_path(&options.scene_path).map_err(|error| error.to_string())?;

    if let Some(RenameNodeOptions { node_id, new_name }) = &options.rename_node {
        app.rename_node(node_id, new_name.clone())
            .map_err(|error| error.to_string())?;
    }

    if let Some(NodePositionEdit { node_id, x, y }) = &options.set_position {
        app.set_position(node_id, *x, *y)
            .map_err(|error| error.to_string())?;
    }

    if let Some(NodeJsonEdit { node_id, json }) = &options.set_params_json {
        let params = parse_json_object(json, "--set-params-json")?;
        app.replace_node_params(node_id, params)
            .map_err(|error| error.to_string())?;
    }

    if let Some(NodeJsonEdit { node_id, json }) = &options.set_style_json {
        let style = parse_json_object(json, "--set-style-json")?;
        app.replace_node_style(node_id, style)
            .map_err(|error| error.to_string())?;
    }

    if has_edits {
        app.save_to_path(&options.scene_path)
            .map_err(|error| error.to_string())?;
    }

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

fn parse_json_object(
    input: &str,
    flag_name: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|error| format!("{flag_name} expected valid JSON: {error}"))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{flag_name} expected a JSON object"))
}
