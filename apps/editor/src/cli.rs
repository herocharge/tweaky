use std::path::PathBuf;

pub struct CliOptions {
    pub scene_path: PathBuf,
    pub mock_prompt: Option<String>,
    pub write_generated_path: Option<PathBuf>,
    pub export_path: Option<PathBuf>,
    pub dump_view_model: bool,
    pub rename_node: Option<RenameNodeOptions>,
    pub set_position: Option<NodePositionEdit>,
    pub set_params_json: Option<NodeJsonEdit>,
    pub set_style_json: Option<NodeJsonEdit>,
    pub qt_shell_requested: bool,
}

pub struct RenameNodeOptions {
    pub node_id: String,
    pub new_name: String,
}

pub struct NodeJsonEdit {
    pub node_id: String,
    pub json: String,
}

pub struct NodePositionEdit {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let _program = args.next();

        let mut scene_path = PathBuf::from("examples/basic_poster.vsd.json");
        let mut mock_prompt = None;
        let mut write_generated_path = None;
        let mut export_path = None;
        let mut dump_view_model = false;
        let mut rename_node = None;
        let mut set_position = None;
        let mut set_params_json = None;
        let mut set_style_json = None;
        let mut qt_shell_requested = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--export" => {
                    let path = args
                        .next()
                        .ok_or_else(|| "--export requires a file path".to_string())?;
                    export_path = Some(PathBuf::from(path));
                }
                "--mock-prompt" => {
                    let prompt = args
                        .next()
                        .ok_or_else(|| "--mock-prompt requires a prompt string".to_string())?;
                    mock_prompt = Some(prompt);
                }
                "--write-generated" => {
                    let path = args
                        .next()
                        .ok_or_else(|| "--write-generated requires a file path".to_string())?;
                    write_generated_path = Some(PathBuf::from(path));
                }
                "--qt" => {
                    qt_shell_requested = true;
                }
                "--dump-view-model" => {
                    dump_view_model = true;
                }
                "--rename-node" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--rename-node requires a node id".to_string())?;
                    let new_name = args
                        .next()
                        .ok_or_else(|| "--rename-node requires a new name".to_string())?;
                    rename_node = Some(RenameNodeOptions { node_id, new_name });
                }
                "--set-position" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--set-position requires a node id".to_string())?;
                    let x = args
                        .next()
                        .ok_or_else(|| "--set-position requires an x value".to_string())?
                        .parse::<f64>()
                        .map_err(|error| format!("invalid x value for --set-position: {error}"))?;
                    let y = args
                        .next()
                        .ok_or_else(|| "--set-position requires a y value".to_string())?
                        .parse::<f64>()
                        .map_err(|error| format!("invalid y value for --set-position: {error}"))?;
                    set_position = Some(NodePositionEdit { node_id, x, y });
                }
                "--set-params-json" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--set-params-json requires a node id".to_string())?;
                    let json = args
                        .next()
                        .ok_or_else(|| "--set-params-json requires a json object".to_string())?;
                    set_params_json = Some(NodeJsonEdit { node_id, json });
                }
                "--set-style-json" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--set-style-json requires a node id".to_string())?;
                    let json = args
                        .next()
                        .ok_or_else(|| "--set-style-json requires a json object".to_string())?;
                    set_style_json = Some(NodeJsonEdit { node_id, json });
                }
                "--help" | "-h" => {
                    return Err(Self::usage());
                }
                value if value.starts_with("--") => {
                    return Err(format!("unknown option: {value}\n\n{}", Self::usage()));
                }
                value => {
                    scene_path = PathBuf::from(value);
                }
            }
        }

        Ok(Self {
            scene_path,
            mock_prompt,
            write_generated_path,
            export_path,
            dump_view_model,
            rename_node,
            set_position,
            set_params_json,
            set_style_json,
            qt_shell_requested,
        })
    }

    pub fn usage() -> String {
        [
            "Usage:",
            "  cargo run -p editor -- [scene-path] [--mock-prompt text] [--write-generated output.vsd.json] [--export output.png] [--dump-view-model] [--rename-node node-id new-name] [--set-position node-id x y] [--set-params-json node-id json] [--set-style-json node-id json] [--qt]",
            "",
            "Defaults:",
            "  scene-path = examples/basic_poster.vsd.json",
            "",
            "Notes:",
            "  --dump-view-model prints the editor-facing JSON payload used by the Qt shell",
            "  --mock-prompt routes through the canned AI adapter instead of reading an input scene path",
            "  --write-generated saves the AI-produced document before continuing",
            "  edit flags write changes back to the scene path before continuing",
            "  json flags expect a full JSON object string like {\"fill\":\"#dd6b42\"}",
        ]
        .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::CliOptions;

    #[test]
    fn parses_default_scene_path() {
        let options = CliOptions::parse(vec!["editor".to_string()]).expect("parse should work");
        assert_eq!(
            options.scene_path.to_string_lossy(),
            "examples/basic_poster.vsd.json"
        );
    }

    #[test]
    fn parses_export_and_qt_flags() {
        let options = CliOptions::parse(vec![
            "editor".to_string(),
            "examples/shapes_study.vsd.json".to_string(),
            "--export".to_string(),
            "out.png".to_string(),
            "--qt".to_string(),
        ])
        .expect("parse should work");

        assert_eq!(
            options.scene_path.to_string_lossy(),
            "examples/shapes_study.vsd.json"
        );
        assert_eq!(
            options
                .export_path
                .expect("export path should exist")
                .to_string_lossy(),
            "out.png"
        );
        assert!(!options.dump_view_model);
        assert!(options.rename_node.is_none());
        assert!(options.set_position.is_none());
        assert!(options.set_params_json.is_none());
        assert!(options.set_style_json.is_none());
        assert!(options.qt_shell_requested);
        assert!(options.mock_prompt.is_none());
        assert!(options.write_generated_path.is_none());
    }

    #[test]
    fn parses_dump_view_model_flag() {
        let options =
            CliOptions::parse(vec!["editor".to_string(), "--dump-view-model".to_string()])
                .expect("parse should work");

        assert!(options.dump_view_model);
    }

    #[test]
    fn parses_rename_node_flag() {
        let options = CliOptions::parse(vec![
            "editor".to_string(),
            "examples/basic_poster.vsd.json".to_string(),
            "--rename-node".to_string(),
            "headline".to_string(),
            "Title Block".to_string(),
        ])
        .expect("parse should work");

        let rename = options.rename_node.expect("rename options should exist");
        assert_eq!(rename.node_id, "headline");
        assert_eq!(rename.new_name, "Title Block");
    }

    #[test]
    fn parses_position_and_json_flags() {
        let options = CliOptions::parse(vec![
            "editor".to_string(),
            "examples/basic_poster.vsd.json".to_string(),
            "--set-position".to_string(),
            "headline".to_string(),
            "320".to_string(),
            "360".to_string(),
            "--set-params-json".to_string(),
            "headline".to_string(),
            "{\"text\":\"JSON MODE\"}".to_string(),
            "--set-style-json".to_string(),
            "headline".to_string(),
            "{\"fill\":\"#112233\"}".to_string(),
        ])
        .expect("parse should work");

        let position = options.set_position.expect("position edit should exist");
        assert_eq!(position.node_id, "headline");
        assert_eq!(position.x, 320.0);
        assert_eq!(position.y, 360.0);

        let params = options
            .set_params_json
            .expect("params json edit should exist");
        assert_eq!(params.node_id, "headline");
        assert_eq!(params.json, "{\"text\":\"JSON MODE\"}");

        let style = options
            .set_style_json
            .expect("style json edit should exist");
        assert_eq!(style.node_id, "headline");
        assert_eq!(style.json, "{\"fill\":\"#112233\"}");
    }

    #[test]
    fn parses_mock_prompt_flags() {
        let options = CliOptions::parse(vec![
            "editor".to_string(),
            "--mock-prompt".to_string(),
            "a drawing of a pelican riding a bicycle".to_string(),
            "--write-generated".to_string(),
            "out.vsd.json".to_string(),
        ])
        .expect("parse should work");

        assert_eq!(
            options.mock_prompt.as_deref(),
            Some("a drawing of a pelican riding a bicycle")
        );
        assert_eq!(
            options
                .write_generated_path
                .expect("generated path should exist")
                .to_string_lossy(),
            "out.vsd.json"
        );
    }
}
