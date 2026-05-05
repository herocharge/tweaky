use std::path::PathBuf;

pub struct CliOptions {
    pub scene_path: PathBuf,
    pub export_path: Option<PathBuf>,
    pub dump_view_model: bool,
    pub rename_node: Option<RenameNodeOptions>,
    pub qt_shell_requested: bool,
}

pub struct RenameNodeOptions {
    pub node_id: String,
    pub new_name: String,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let _program = args.next();

        let mut scene_path = PathBuf::from("examples/basic_poster.vsd.json");
        let mut export_path = None;
        let mut dump_view_model = false;
        let mut rename_node = None;
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
            qt_shell_requested,
        })
    }

    pub fn usage() -> String {
        [
            "Usage:",
            "  cargo run -p editor -- [scene-path] [--export output.png] [--dump-view-model] [--rename-node node-id new-name] [--qt]",
            "",
            "Defaults:",
            "  scene-path = examples/basic_poster.vsd.json",
            "",
            "Notes:",
            "  --dump-view-model prints the editor-facing JSON payload used by the Qt shell",
            "  --rename-node applies a rename, writes it back to the scene path, then continues",
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
}
