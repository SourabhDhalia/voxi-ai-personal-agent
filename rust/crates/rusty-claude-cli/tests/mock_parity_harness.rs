use std::path::Path;
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root")
}

#[test]
fn parity_harness_reports_success_for_the_current_checkout() {
    let root = repo_root();
    let output = Command::new("bash")
        .arg(root.join("rust/scripts/run_mock_parity_harness.sh"))
        .arg("--json")
        .current_dir(root)
        .output()
        .expect("parity harness should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("\"status\": \"pass\""));
    assert!(stdout.contains("\"workspace_members\""));
}
