use super::*;

fn run_spec(entry_point: &str) -> RunSpec {
    RunSpec {
        entry_point: entry_point.to_string(),
        vm_args: "-Xmx128m -ea".to_string(),
        program_args: "foo bar".to_string(),
        env: vec![("MY_ENV".to_string(), "hello".to_string())],
        working_dir: None,
    }
}

#[test]
fn launch_carries_vm_args_classpath_entry_point_program_args_and_env() {
    let spec = launch_spec(
        Path::new("/project"),
        &run_spec("com.example.Main"),
        &[PathBuf::from("/fake/classes"), PathBuf::from("/fake/dep.jar")],
    );

    let separator = if cfg!(windows) { ";" } else { ":" };
    assert_eq!(spec.program, PathBuf::from("java"));
    assert_eq!(
        spec.args,
        vec![
            "-Xmx128m".to_string(),
            "-ea".to_string(),
            "-cp".to_string(),
            format!("/fake/classes{separator}/fake/dep.jar"),
            "com.example.Main".to_string(),
            "foo".to_string(),
            "bar".to_string(),
        ]
    );
    assert_eq!(spec.env, vec![("MY_ENV".to_string(), "hello".to_string())]);
}

#[test]
fn launch_runs_in_the_project_root_unless_the_configuration_names_a_directory() {
    let mut spec = run_spec("com.example.Main");
    assert_eq!(
        launch_spec(Path::new("/project"), &spec, &[]).cwd,
        PathBuf::from("/project")
    );

    spec.working_dir = Some(PathBuf::from("/elsewhere"));
    assert_eq!(
        launch_spec(Path::new("/project"), &spec, &[]).cwd,
        PathBuf::from("/elsewhere")
    );
}

#[test]
fn a_build_tool_this_extension_does_not_own_has_no_launch_and_no_classpath() {
    let root = Path::new("/project");
    assert!(command("bazel", root, &run_spec("com.example.Main")).is_none());
    assert!(runtime_classpath("bazel", root).is_none());
    assert!(analysis_classpath("bazel", root).is_empty());
}
