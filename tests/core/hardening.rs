//! Milestone 12: the properties that make "local-first" trustworthy, checked as
//! code so they cannot quietly drift. See docs/PRIVACY_REVIEW.md.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn repo_root() -> PathBuf {
    manifest_dir().join("..")
}
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
fn files_under(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = vec![];
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_under(&path, extension));
        } else if path.extension().is_some_and(|e| e == extension) {
            found.push(path);
        }
    }
    found.sort();
    found
}
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---- logging ---------------------------------------------------------------------------

const LOGGING: [&str; 9] = [
    "println!",
    "eprintln!",
    "print!(",
    "eprint!(",
    "dbg!",
    "log::",
    "tracing::",
    "info!(",
    "warn!(",
];

/// Every logging call in the Rust source. Rust source files only, not tests.
fn logging_calls() -> Vec<(String, String)> {
    let src = manifest_dir().join("src");
    let mut calls = vec![];
    for file in files_under(&src, "rs") {
        for line in read(&file).lines() {
            if LOGGING.iter().any(|m| line.contains(m)) {
                let rel = file
                    .strip_prefix(&src)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                calls.push((rel, squash(line)));
            }
        }
    }
    calls
}

/// The complete list. Adding a log line means adding it here on purpose, having
/// asked whether it could reveal what someone was doing.
const ALLOWED_LOGGING: [(&str, &str); 2] = [
    (
        "interventions/window.rs",
        r#"eprintln!("could not open the check-in window: {e}");"#,
    ),
    (
        "interventions/window.rs",
        r#"eprintln!("could not open the pause window: {e}");"#,
    ),
];

#[test]
fn the_only_log_lines_are_the_reviewed_ones() {
    let found: BTreeSet<(String, String)> = logging_calls().into_iter().collect();
    let allowed: BTreeSet<(String, String)> = ALLOWED_LOGGING
        .iter()
        .map(|(f, l)| (f.to_string(), squash(l)))
        .collect();
    assert_eq!(
        found, allowed,
        "a log line was added or removed: review it against docs/PRIVACY_REVIEW.md"
    );
}

#[test]
fn the_reviewed_log_lines_print_only_the_error_and_nothing_about_the_user() {
    for (_, line) in logging_calls() {
        assert!(line.ends_with(r#": {e}");"#), "{line}");
        for sensitive in [
            "bundle",
            "app_name",
            "application",
            "title",
            "timestamp",
            "session",
            "text",
            "interest",
        ] {
            assert!(!line.contains(sensitive), "{sensitive:?} in {line}");
        }
    }
}

#[test]
fn the_frontend_never_uses_the_console() {
    for file in ["ts", "tsx"]
        .iter()
        .flat_map(|e| files_under(&repo_root().join("src"), e))
    {
        assert!(!read(&file).contains("console."), "{}", file.display());
    }
}

// ---- network -----------------------------------------------------------------------------

const NETWORK_CRATES: [&str; 14] = [
    "reqwest",
    "hyper",
    "ureq",
    "isahc",
    "curl",
    "attohttpc",
    "surf",
    "awc",
    "native-tls",
    "rustls",
    "openssl",
    "h2",
    "tungstenite",
    "tokio-tungstenite",
];

#[test]
fn the_app_declares_no_network_dependency() {
    let manifest = read(&manifest_dir().join("Cargo.toml"));
    for name in NETWORK_CRATES {
        assert!(
            !manifest
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{name} "))
                    || l.trim_start().starts_with(&format!("{name}="))),
            "{name} must not be a dependency"
        );
    }
}

fn tauri_conf() -> serde_json::Value {
    serde_json::from_str(&read(&manifest_dir().join("tauri.conf.json"))).unwrap()
}

fn csp_directives() -> Vec<(String, Vec<String>)> {
    let csp = tauri_conf()["app"]["security"]["csp"]
        .as_str()
        .expect("a CSP is set")
        .to_string();
    csp.split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| {
            let mut parts = d.split_whitespace().map(String::from);
            (parts.next().unwrap(), parts.collect())
        })
        .collect()
}
fn directive(name: &str) -> Option<Vec<String>> {
    csp_directives()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v)
}

#[test]
fn the_content_security_policy_allows_only_the_apps_own_origin() {
    assert_eq!(directive("default-src"), Some(vec!["'self'".to_string()]));
    for (name, sources) in csp_directives() {
        for source in &sources {
            for banned in [
                "http:",
                "https:",
                "ws:",
                "wss:",
                "*",
                "'unsafe-eval'",
                "blob:",
                "filesystem:",
            ] {
                assert!(!source.contains(banned), "{name}: {source}");
            }
        }
    }
}

#[test]
fn scripts_cannot_be_inline_and_nothing_can_be_embedded_or_submitted() {
    let scripts = directive("script-src").unwrap_or_else(|| directive("default-src").unwrap());
    assert!(
        !scripts.iter().any(|s| s.contains("unsafe-inline")),
        "no inline scripts"
    );
    assert_eq!(directive("object-src"), Some(vec!["'none'".to_string()]));
    assert_eq!(directive("base-uri"), Some(vec!["'self'".to_string()]));
    assert_eq!(directive("form-action"), Some(vec!["'none'".to_string()]));
    assert!(
        directive("connect-src").is_none_or(|s| s == ["'self'"]),
        "no outside connections"
    );
}

#[test]
fn the_shell_does_not_open_extra_doors() {
    let conf = tauri_conf();
    let security = &conf["app"]["security"];
    assert!(security
        .get("dangerousDisableAssetCspModification")
        .is_none());
    assert!(security.get("dangerousRemoteDomainIpcAccess").is_none());
    assert!(
        conf["app"]
            .get("withGlobalTauri")
            .is_none_or(|v| v == false),
        "no global Tauri object"
    );
    assert!(security["assetProtocol"].is_null() || security["assetProtocol"]["enable"] == false);
    // tauri.conf.json's `plugins` key is for plugin *configuration*, not for
    // declaring one; this app configures none.
    assert!(
        conf["plugins"].is_null() || conf["plugins"].as_object().is_some_and(|p| p.is_empty()),
        "no plugin configuration"
    );
    // The only plugin used is global-shortcut ("I'm on fire", Milestone 14), added
    // in Rust and called only from Rust: no capability may grant it to a webview.
    let cargo_toml = read(&manifest_dir().join("Cargo.toml"));
    let plugin_deps: Vec<&str> = cargo_toml
        .lines()
        .filter(|l| l.trim_start().starts_with("tauri-plugin-"))
        .map(|l| l.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(
        plugin_deps,
        ["tauri-plugin-global-shortcut"],
        "no plugin beyond the reviewed one"
    );
    for file in ["default.json", "intervention.json", "pause.json"] {
        let json: serde_json::Value =
            serde_json::from_str(&read(&manifest_dir().join("capabilities").join(file))).unwrap();
        for p in json["permissions"].as_array().unwrap() {
            assert!(
                !p.as_str().unwrap().starts_with("global-shortcut"),
                "{file} must not be granted the shortcut plugin"
            );
        }
    }
}

#[test]
fn no_window_loads_anything_from_outside_the_app() {
    let window_code = read(&manifest_dir().join("src/interventions/window.rs"));
    assert!(!window_code.contains("WebviewUrl::External"));
    assert_eq!(
        window_code.matches("WebviewUrl::App").count(),
        2,
        "the two small windows, both local"
    );
    for url in ["http://", "https://"] {
        let conf = read(&manifest_dir().join("tauri.conf.json"));
        // Only the dev server address and the schema link may appear.
        for line in conf.lines().filter(|l| l.contains(url)) {
            assert!(
                line.contains("localhost") || line.contains("schema.tauri.app"),
                "{line}"
            );
        }
    }
}

// ---- the review document stays true ------------------------------------------------------------

#[test]
fn the_review_document_lists_exactly_the_permissions_the_capabilities_grant() {
    let doc = read(&repo_root().join("docs/PRIVACY_REVIEW.md"));
    for file in ["default.json", "intervention.json", "pause.json"] {
        assert!(doc.contains(file), "{file} is not described");
        let json: serde_json::Value =
            serde_json::from_str(&read(&manifest_dir().join("capabilities").join(file))).unwrap();
        for permission in json["permissions"].as_array().unwrap() {
            let p = permission.as_str().unwrap();
            assert!(
                doc.contains(&format!("`{p}`")),
                "{file}: {p} is granted but not in the review"
            );
        }
    }
}

#[test]
fn the_review_document_does_not_mention_permissions_that_are_not_granted() {
    let doc = read(&repo_root().join("docs/PRIVACY_REVIEW.md"));
    let mut granted = BTreeSet::new();
    for file in ["default.json", "intervention.json", "pause.json"] {
        let json: serde_json::Value =
            serde_json::from_str(&read(&manifest_dir().join("capabilities").join(file))).unwrap();
        granted.extend(
            json["permissions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap().to_string()),
        );
    }
    for token in doc.split('`').skip(1).step_by(2) {
        if token.starts_with("allow-") || token.starts_with("core:") {
            assert!(
                granted.contains(token),
                "the review lists {token}, which nothing grants"
            );
        }
    }
}

#[test]
fn the_review_document_matches_the_logging_and_the_window_count() {
    let doc = read(&repo_root().join("docs/PRIVACY_REVIEW.md"));
    assert!(doc.contains("exactly two log lines"));
    assert_eq!(ALLOWED_LOGGING.len(), 2);
    assert!(doc.contains("scripts/check-no-network.sh"));
    assert!(repo_root().join("scripts/check-no-network.sh").exists());
}
