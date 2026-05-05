use std::path::PathBuf;

pub struct CliOptions {
    pub scene_path: PathBuf,
    pub export_path: Option<PathBuf>,
    pub dump_view_model: bool,
    pub rename_node: Option<RenameNodeOptions>,
    pub set_text: Option<NodeStringEdit>,
    pub set_fill: Option<NodeStringEdit>,
    pub qt_shell_requested: bool,
}

pub struct RenameNodeOptions {
    pub node_id: String,
    pub new_name: String,
}

pub struct NodeStringEdit {
    pub node_id: String,
    pub value: String,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let _program = args.next();

        let mut scene_path = PathBuf::from("examples/basic_poster.vsd.json");
        let mut export_path = None;
        let mut dump_view_model = false;
        let mut rename_node = None;
        let mut set_text = None;
        let mut set_fill = None;
        let mut qt_shell_requested = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--export" => {
                    let path = args
                        .next()
                        .ok_or_else(|| "--export requires a file path".to_string())?;
                    export_path = Some(PathBuf::from(path));
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
                "--set-text" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--set-text requires a node id".to_string())?;
                    let value = args
                        .next()
                        .ok_or_else(|| "--set-text requires a text value".to_string())?;
                    set_text = Some(NodeStringEdit { node_id, value });
                }
                "--set-fill" => {
                    let node_id = args
                        .next()
                        .ok_or_else(|| "--set-fill requires a node id".to_string())?;
                    let value = args
                        .next()
                        .ok_or_else(|| "--set-fill requires a fill value".to_string())?;
                    set_fill = Some(NodeStringEdit { node_id, value });
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
            export_path,
            dump_view_model,
            rename_node,
            set_text,
            set_fill,
            qt_shell_requested,
        })
    }

    pub fn usage() -> String {
        [
            "Usage:",
            "  cargo run -p editor -- [scene-path] [--export output.png] [--dump-view-model] [--rename-node node-id new-name] [--set-text node-id text] [--set-fill node-id color] [--qt]",
            "",
            "Defaults:",
            "  scene-path = examples/basic_poster.vsd.json",
            "",
            "Notes:",
            "  --dump-view-model prints the editor-facing JSON payload used by the Qt shell",
            "  edit flags write changes back to the scene path before continuing",
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
        assert!(options.set_text.is_none());
        assert!(options.set_fill.is_none());
        assert!(options.qt_shell_requested);
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
    fn parses_text_and_fill_flags() {
        let options = CliOptions::parse(vec![
            "editor".to_string(),
            "examples/basic_poster.vsd.json".to_string(),
            "--set-text".to_string(),
            "headline".to_string(),
            "MAKE IT YOURS".to_string(),
            "--set-fill".to_string(),
            "headline".to_string(),
            "#112233".to_string(),
        ])
        .expect("parse should work");

        let text = options.set_text.expect("text edit should exist");
        assert_eq!(text.node_id, "headline");
        assert_eq!(text.value, "MAKE IT YOURS");

        let fill = options.set_fill.expect("fill edit should exist");
        assert_eq!(fill.node_id, "headline");
        assert_eq!(fill.value, "#112233");
    }
}
