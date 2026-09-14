use std::env;
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

static VIM_RE: OnceLock<Regex> = OnceLock::new();

fn vim_re() -> &'static Regex {
    VIM_RE.get_or_init(|| Regex::new(r"^g?(view|l?n?vim?x?)(diff)?$").expect("vim regex"))
}

fn direction(dir: &str) -> Option<(&'static str, i64)> {
    match dir {
        "left" => Some(("ctrl+h", -1)),
        "down" => Some(("ctrl+j", 1)),
        "up" => Some(("ctrl+k", -1)),
        "right" => Some(("ctrl+l", 1)),
        _ => None,
    }
}

fn herdr_bin() -> String {
    env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

fn run_herdr(args: &[&str]) -> Option<Value> {
    let out = Command::new(herdr_bin()).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

fn pane_id() -> Option<String> {
    for key in ["HERDR_PANE_ID", "HERDR_ACTIVE_PANE_ID"] {
        if let Ok(value) = env::var(key) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn base_name(command: &str) -> String {
    command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .to_lowercase()
}

fn identities(proc: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["name", "argv0"] {
        if let Some(s) = proc.get(key).and_then(Value::as_str) {
            out.push(base_name(s));
        }
    }
    if let Some(first) = proc
        .get("argv")
        .and_then(|a| a.get(0))
        .and_then(Value::as_str)
    {
        out.push(base_name(first));
    }
    out
}

fn is_vim(proc: &Value) -> bool {
    identities(proc).iter().any(|id| vim_re().is_match(id))
}

fn is_passthrough_with(proc: &Value, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    let Ok(matcher) = Regex::new(pattern) else {
        return false;
    };
    identities(proc).iter().any(|id| matcher.is_match(id))
}

fn is_passthrough(proc: &Value) -> bool {
    is_passthrough_with(
        proc,
        &env::var("HERDR_NAV_PASSTHROUGH_RE").unwrap_or_default(),
    )
}

fn level_vim(key: &str, pane: Option<&str>) -> bool {
    let Some(pane) = pane else {
        return false;
    };
    let Some(data) = run_herdr(&["pane", "process-info", "--pane", pane]) else {
        return false;
    };
    let Some(procs) = data
        .pointer("/result/process_info/foreground_processes")
        .and_then(Value::as_array)
    else {
        return false;
    };
    if procs.iter().any(|p| is_vim(p) || is_passthrough(p)) {
        run_herdr(&["pane", "send-keys", pane, key]);
        return true;
    }
    false
}

fn focus_changed(data: &Value) -> bool {
    data.pointer("/result/focus/changed")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn level_pane(dir: &str, pane: Option<&str>) -> bool {
    let args: Vec<&str> = match pane {
        Some(p) => vec!["pane", "focus", "--direction", dir, "--pane", p],
        None => vec!["pane", "focus", "--direction", dir, "--current"],
    };
    match run_herdr(&args) {
        Some(data) => focus_changed(&data),
        None => false,
    }
}

enum Pick {
    Id(String),
    End,
    Miss,
}

fn strip_pick(items: &[Value], id_key: &str, step: i64, wrap: bool) -> Pick {
    if items.is_empty() {
        return Pick::Miss;
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| items[i].get("number").and_then(Value::as_u64).unwrap_or(0));
    let Some(rank) = order.iter().position(|&i| {
        items[i]
            .get("focused")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }) else {
        return Pick::Miss;
    };
    let target = rank as i64 + step;
    let n = order.len() as i64;
    let slot = if target < 0 || target >= n {
        if !wrap {
            return Pick::End;
        }
        ((target % n + n) % n) as usize
    } else {
        target as usize
    };
    match items[order[slot]].get(id_key).and_then(Value::as_str) {
        Some(id) => Pick::Id(id.to_string()),
        None => Pick::Miss,
    }
}

fn pane_workspace(pane: Option<&str>) -> Option<String> {
    if let Some(pane) = pane {
        let data = run_herdr(&["pane", "list"])?;
        let panes = data.pointer("/result/panes").and_then(Value::as_array)?;
        return panes
            .iter()
            .find(|p| p.get("pane_id").and_then(Value::as_str) == Some(pane))
            .and_then(|p| p.get("workspace_id"))
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    run_herdr(&["pane", "current", "--current"])?
        .pointer("/result/pane/workspace_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn level_tab(step: i64, pane: Option<&str>) -> bool {
    let Some(workspace) = pane_workspace(pane) else {
        return false;
    };
    let Some(data) = run_herdr(&["tab", "list", "--workspace", &workspace]) else {
        return false;
    };
    let Some(tabs) = data.pointer("/result/tabs").and_then(Value::as_array) else {
        return false;
    };
    if tabs.len() < 2 {
        return false;
    }
    match strip_pick(tabs, "tab_id", step, true) {
        Pick::Id(id) => run_herdr(&["tab", "focus", &id]).is_some(),
        _ => false,
    }
}

fn level_space(step: i64) -> bool {
    let Some(data) = run_herdr(&["workspace", "list"]) else {
        return false;
    };
    let Some(spaces) = data.pointer("/result/workspaces").and_then(Value::as_array) else {
        return false;
    };
    if spaces.len() < 2 {
        return false;
    }
    let Pick::Id(id) = strip_pick(spaces, "workspace_id", step, true) else {
        return false;
    };
    run_herdr(&["workspace", "focus", &id]).is_some()
}

fn cross(dir: &str, step: i64, pane: Option<&str>) {
    if level_pane(dir, pane) {
        return;
    }
    match dir {
        "up" | "down" => {
            level_space(step);
        }
        _ => {
            level_tab(step, pane);
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next().unwrap_or_default();
    let (forward, dir) = if first == "cross" {
        (false, args.next().unwrap_or_default())
    } else {
        (true, first)
    };
    let Some((key, step)) = direction(&dir) else {
        eprintln!("usage: herdr-smart-nav [cross] left|right|up|down");
        std::process::exit(2);
    };
    let pane = pane_id();
    if forward && level_vim(key, pane.as_deref()) {
        return;
    }
    cross(&dir, step, pane.as_deref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_nvim_by_name() {
        assert!(is_vim(&json!({"name": "nvim"})));
    }

    #[test]
    fn detects_vim_by_argv_path() {
        assert!(is_vim(
            &json!({"name": "?", "argv0": "/usr/local/bin/nvim"})
        ));
        assert!(is_vim(&json!({"name": "?", "argv": ["vim", "file"]})));
        assert!(is_vim(&json!({"name": "view"})));
        assert!(is_vim(&json!({"name": "nvimdiff"})));
    }

    #[test]
    fn claude_version_name_is_not_vim() {
        let proc = json!({"name": "2.1.268", "argv0": "claude", "argv": ["claude"]});
        assert!(!is_vim(&proc));
    }

    #[test]
    fn shell_is_not_vim() {
        let proc = json!({"name": "node", "argv0": "node", "argv": ["node", "x.js"]});
        assert!(!is_vim(&proc));
    }

    #[test]
    fn passthrough_empty_pattern_is_off() {
        assert!(!is_passthrough_with(&json!({"name": "lazygit"}), ""));
    }

    #[test]
    fn passthrough_regex_match() {
        let proc = json!({"name": "lazygit"});
        assert!(is_passthrough_with(&proc, "^(lazygit|k9s)$"));
        assert!(!is_passthrough_with(
            &json!({"name": "nvim"}),
            "^(lazygit|k9s)$"
        ));
    }

    #[test]
    fn passthrough_bad_regex_is_safe() {
        assert!(!is_passthrough_with(
            &json!({"name": "lazygit"}),
            "(unclosed"
        ));
    }

    fn tabs() -> Vec<Value> {
        vec![
            json!({"number": 1, "tab_id": "w:t1", "focused": false}),
            json!({"number": 2, "tab_id": "w:t2", "focused": true}),
            json!({"number": 3, "tab_id": "w:t3", "focused": false}),
        ]
    }

    #[test]
    fn strip_middle_steps() {
        let t = tabs();
        assert!(matches!(strip_pick(&t, "tab_id", 1, false), Pick::Id(id) if id == "w:t3"));
        assert!(matches!(strip_pick(&t, "tab_id", -1, false), Pick::Id(id) if id == "w:t1"));
    }

    #[test]
    fn strip_edges_do_not_wrap() {
        let last = vec![
            json!({"number": 1, "tab_id": "w:t1", "focused": false}),
            json!({"number": 2, "tab_id": "w:t2", "focused": true}),
        ];
        assert!(matches!(strip_pick(&last, "tab_id", 1, false), Pick::End));
        let first = vec![
            json!({"number": 1, "tab_id": "w:t1", "focused": true}),
            json!({"number": 2, "tab_id": "w:t2", "focused": false}),
        ];
        assert!(matches!(strip_pick(&first, "tab_id", -1, false), Pick::End));
    }

    #[test]
    fn strip_wrap_spaces() {
        let ws = vec![
            json!({"number": 1, "workspace_id": "a", "focused": true}),
            json!({"number": 2, "workspace_id": "b", "focused": false}),
        ];
        assert!(matches!(strip_pick(&ws, "workspace_id", -1, true), Pick::Id(id) if id == "b"));
        assert!(matches!(strip_pick(&ws, "workspace_id", 1, true), Pick::Id(id) if id == "b"));
    }

    #[test]
    fn strip_wrap_from_last() {
        let ws = vec![
            json!({"number": 1, "workspace_id": "a", "focused": false}),
            json!({"number": 2, "workspace_id": "b", "focused": true}),
        ];
        assert!(matches!(strip_pick(&ws, "workspace_id", 1, true), Pick::Id(id) if id == "a"));
    }

    #[test]
    fn strip_no_focused_is_miss() {
        let t = vec![json!({"number": 1, "tab_id": "w:t1", "focused": false})];
        assert!(matches!(strip_pick(&t, "tab_id", 1, false), Pick::Miss));
        let empty: Vec<Value> = vec![];
        assert!(matches!(strip_pick(&empty, "tab_id", 1, true), Pick::Miss));
    }

    #[test]
    fn focus_changed_parsing() {
        assert!(focus_changed(
            &json!({"result": {"focus": {"changed": true}}})
        ));
        assert!(!focus_changed(
            &json!({"result": {"focus": {"changed": false}}})
        ));
        assert!(focus_changed(&json!({"result": {"focus": {}}})));
        assert!(focus_changed(&json!({})));
    }
}
