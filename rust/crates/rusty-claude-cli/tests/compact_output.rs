use std::io::Cursor;

use rusty_claude_cli::input::OutputFormat;
use rusty_claude_cli::run_with_stdio;

#[test]
fn compact_output_stays_single_line_with_piped_input() {
    let mut input = Cursor::new(b"stdin details".to_vec());
    let mut output = Vec::new();
    let outcome = run_with_stdio(
        vec![
            "rusty-claude-cli".to_string(),
            "--compact".to_string(),
            "draft".to_string(),
            "summary".to_string(),
        ],
        &mut input,
        false,
        &mut output,
    )
    .expect("cli run should succeed");

    assert_eq!(outcome.output_format, OutputFormat::Compact);
    assert_eq!(
        outcome.input.merged_prompt.as_deref(),
        Some("draft summary\n\nstdin details")
    );

    let rendered = String::from_utf8(output).expect("utf8");
    assert_eq!(rendered.trim().lines().count(), 1);
    assert!(rendered.contains("mode=prompt"));
    assert!(rendered.contains("prompt=\"draft summary"));
    assert!(rendered.contains("stdin details\""));
}
