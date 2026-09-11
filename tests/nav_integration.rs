use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

const PROCS_NVIM: &str =
    r#"{"result":{"process_info":{"foreground_processes":[{"name":"nvim","argv0":"nvim"}]}}}"#;
const PROCS_SHELL: &str = r#"{"result":{"process_info":{"foreground_processes":[{"name":"zsh","argv0":"-zsh","argv":["zsh"]}]}}}"#;
const PROCS_CLAUDE: &str = r#"{"result":{"process_info":{"foreground_processes":[{"name":"2.1.268","argv0":"claude","argv":["claude"]}]}}}"#;
const FOCUS_MOVED: &str = r#"{"result":{"focus":{"changed":true}}}"#;
const FOCUS_EDGE: &str = r#"{"result":{"focus":{"changed":false,"reason":"no_neighbor"}}}"#;
const PANES_W9: &str =
    r#"{"result":{"panes":[{"pane_id":"w9:p1","workspace_id":"w9","tab_id":"w9:t1"}]}}"#;
const TABS_MID: &str = r#"{"result":{"tabs":[
  {"number":1,"tab_id":"w9:t1","focused":false},
  {"number":2,"tab_id":"w9:t2","focused":true},
  {"number":3,"tab_id":"w9:t3","focused":false}
]}}"#;
const TABS_LAST: &str = r#"{"result":{"tabs":[
  {"number":1,"tab_id":"w9:t1","focused":false},
  {"number":2,"tab_id":"w9:t2","focused":true}
]}}"#;
const SPACES_TWO: &str = r#"{"result":{"workspaces":[
  {"number":1,"workspace_id":"w9","focused":true},
  {"number":2,"workspace_id":"wB","focused":false}
]}}"#;
const SPACES_ONE: &str = r#"{"result":{"workspaces":[
  {"number":1,"workspace_id":"w9","focused":true}
]}}"#;

fn bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_herdr_smart_nav") {
        return PathBuf::from(path);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["debug", "release"] {
        let path = root.join("target").join(profile).join("herdr-smart-nav");
        if path.exists() {
            return path;
        }
    }
    panic!("herdr-smart-nav binary not built; run `cargo build` first");
}

fn run_nav(args: &[&str], vars: &[(&str, &str)]) -> (i32, String) {
    let stub = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/stub_herdr.sh");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&stub) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&stub, perms);
        }
    }
    let log = env::temp_dir().join(format!(
        "smart-nav-test-{}-{}.log",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let mut cmd = Command::new(bin());
    cmd.args(args)
        .env("HERDR_BIN_PATH", &stub)
        .env("HERDR_PANE_ID", "w9:p1")
        .env("CALL_LOG", &log)
        .env_remove("HERDR_NAV_PASSTHROUGH_RE");
    for (k, v) in vars {
        cmd.env(k, v);
    }
    let code = cmd.status().expect("run binary").code().unwrap_or(-1);
    let text = fs::read_to_string(&log).unwrap_or_default();
    let _ = fs::remove_file(&log);
    (code, text)
}

#[test]
fn vim_pane_forwards_key() {
    let (code, log) = run_nav(
        &["left"],
        &[
            ("STUB_PROCESS_INFO", PROCS_NVIM),
            ("STUB_PANE_LIST", PANES_W9),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("send-keys w9:p1 ctrl+h"), "{log}");
    assert!(!log.contains("focus left"), "{log}");
}

#[test]
fn shell_pane_moves_focus() {
    let (code, log) = run_nav(
        &["left"],
        &[
            ("STUB_PROCESS_INFO", PROCS_SHELL),
            ("STUB_FOCUS", FOCUS_MOVED),
            ("STUB_PANE_LIST", PANES_W9),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("focus left"), "{log}");
    assert!(!log.contains("tab list"), "{log}");
}

#[test]
fn pane_edge_moves_tab() {
    let (code, log) = run_nav(
        &["right"],
        &[
            ("STUB_PROCESS_INFO", PROCS_SHELL),
            ("STUB_FOCUS", FOCUS_EDGE),
            ("STUB_PANE_LIST", PANES_W9),
            ("STUB_TAB_LIST", TABS_MID),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("tab focus w9:t3"), "{log}");
    assert!(log.contains("tab list --workspace w9"), "{log}");
    assert!(!log.contains("workspace list"), "{log}");
}

#[test]
fn tab_edge_moves_space() {
    let (code, log) = run_nav(
        &["right"],
        &[
            ("STUB_PROCESS_INFO", PROCS_SHELL),
            ("STUB_FOCUS", FOCUS_EDGE),
            ("STUB_PANE_LIST", PANES_W9),
            ("STUB_TAB_LIST", TABS_LAST),
            ("STUB_WORKSPACE_LIST", SPACES_TWO),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("workspace focus wB"), "{log}");
}

#[test]
fn tab_focus_failure_falls_to_space() {
    let (code, log) = run_nav(
        &["right"],
        &[
            ("STUB_PROCESS_INFO", PROCS_SHELL),
            ("STUB_FOCUS", FOCUS_EDGE),
            ("STUB_PANE_LIST", PANES_W9),
            ("STUB_TAB_LIST", TABS_MID),
            ("STUB_TAB_FOCUS_FAIL", "1"),
            ("STUB_WORKSPACE_LIST", SPACES_TWO),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("workspace focus wB"), "{log}");
}

#[test]
fn claude_version_name_moves_pane() {
    let (code, log) = run_nav(
        &["left"],
        &[
            ("STUB_PROCESS_INFO", PROCS_CLAUDE),
            ("STUB_FOCUS", FOCUS_MOVED),
            ("STUB_PANE_LIST", PANES_W9),
        ],
    );
    assert_eq!(code, 0);
    assert!(log.contains("focus left"), "{log}");
    assert!(!log.contains("send-keys"), "{log}");
}

#[test]
fn cross_skips_vim_forward() {
    let (code, log) = run_nav(
        &["cross", "left"],
        &[
            ("STUB_PROCESS_INFO", PROCS_NVIM),
            ("STUB_FOCUS", FOCUS_MOVED),
            ("STUB_PANE_LIST", PANES_W9),
        ],
    );
    assert_eq!(code, 0);
    assert!(!log.contains("send-keys"), "{log}");
    assert!(log.contains("focus left"), "{log}");
}

#[test]
fn single_space_stays_quiet() {
    let (code, log) = run_nav(
        &["right"],
        &[
            ("STUB_PROCESS_INFO", PROCS_SHELL),
            ("STUB_FOCUS", FOCUS_EDGE),
            ("STUB_PANE_LIST", PANES_W9),
            ("STUB_TAB_LIST", TABS_LAST),
            ("STUB_WORKSPACE_LIST", SPACES_ONE),
        ],
    );
    assert_eq!(code, 0);
    assert!(!log.contains("workspace focus"), "{log}");
}

#[test]
fn usage_error() {
    let (code, _) = run_nav(&[], &[]);
    assert_eq!(code, 2);
    let (code, _) = run_nav(&["diagonal"], &[]);
    assert_eq!(code, 2);
}
