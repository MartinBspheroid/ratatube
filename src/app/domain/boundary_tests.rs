//! Compile-time-adjacent guard: the domain half must stay free of terminal
//! and rendering types so Phase 2 can host it in a headless daemon.

use std::path::Path;

/// Source files that constitute the domain half.
fn domain_sources() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("src/app/state/domain_state.rs")];
    let dir = root.join("src/app/domain");
    let entries = std::fs::read_dir(&dir).expect("domain dir");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs")
            && path
                .file_name()
                .is_some_and(|name| name != "boundary_tests.rs")
        {
            files.push(path);
        }
    }
    files
}

#[test]
fn domain_half_never_references_ratatui() {
    for file in domain_sources() {
        let source = std::fs::read_to_string(&file).expect("read domain source");
        for (number, line) in source.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);
            assert!(
                !code.contains("ratatui"),
                "{}:{} references ratatui in the domain half: {line}",
                file.display(),
                number + 1
            );
        }
    }
}
