use std::path::PathBuf;

pub struct CliOptions {
    pub scene_path: PathBuf,
    pub export_path: Option<PathBuf>,
    pub qt_shell_requested: bool,
}

impl CliOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let _program = args.next();

        let mut scene_path = PathBuf::from("examples/basic_poster.vsd.json");
        let mut export_path = None;
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
            qt_shell_requested,
        })
    }

    pub fn usage() -> String {
        [
            "Usage:",
            "  cargo run -p editor -- [scene-path] [--export output.png] [--qt]",
            "",
            "Defaults:",
            "  scene-path = examples/basic_poster.vsd.json",
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
        assert!(options.qt_shell_requested);
    }
}
