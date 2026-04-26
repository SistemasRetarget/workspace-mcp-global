//! Quality Gate MCP server — Retarget workspace
//!
//! A Model Context Protocol server (JSON-RPC 2.0 over stdio, spec 2024-11-05)
//! that acts as a *blocking* quality gate for every project in the Retarget
//! workspace. When a check fails, the response is marked `isError: true` so
//! the calling agent knows it must iterate.
//!
//! Tools:
//!   - list-projects  Discover projects under WORKSPACE_ROOT.
//!   - validate       Run typecheck → lint → build; stops at first failure.
//!   - run-tests      Run the project's test suite.
//!   - logs           Tail the last N lines of any log file.
//!   - railway-logs   Tail Railway deploy logs via the Railway CLI.
//!
//! Workspace root: env var `RETARGET_WORKSPACE` overrides the default
//! `~/Desktop/RETARGET-WORKSPACE/PROJECTS`.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde_json::{json, Value};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

mod tools;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "quality-gate-server";
const SERVER_VERSION: &str = "0.3.1";

// Filesystem layout for visual evidence + lessons KB.
// Override via env: WORKSPACE_MCP_HOME (defaults to ~/Documents/workspace-mcp-global)
fn mcp_home() -> PathBuf {
    if let Ok(v) = std::env::var("WORKSPACE_MCP_HOME") {
        return PathBuf::from(v);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join("Documents/workspace-mcp-global");
    }
    PathBuf::from(".")
}
fn evidence_dir() -> PathBuf { mcp_home().join("evidence") }
fn lessons_file() -> PathBuf { mcp_home().join("lessons/lessons.jsonl") }
fn visual_diff_script() -> PathBuf { mcp_home().join("tools/visual-diff.mjs") }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => {
                let resp = error_response(Value::Null, -32700, "Parse error");
                write_line(&mut stdout, &resp).await?;
                continue;
            }
        };

        let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = message.get("id").cloned();
        if id.is_none() {
            continue; // notification — never reply
        }
        let id = id.unwrap();

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": { "tools": tool_definitions() }
            }),
            "tools/call" => handle_tool_call(id, message.get("params")),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            _ => error_response(id, -32601, &format!("Method not found: {}", method)),
        };

        write_line(&mut stdout, &response).await?;
    }
    Ok(())
}

// ─── Tool schemas ───────────────────────────────────────────────────────────

fn tool_definitions() -> Value {
    json!([
        {
            "name": "list-projects",
            "description": "Discover every project under the Retarget workspace root. \
                            Returns an array of { name, path, kind } where kind is one of \
                            'next', 'node', 'rust', 'python', 'unknown'.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "validate",
            "description": "Blocking quality gate: runs typecheck → lint → build in order, \
                            stopping at first failure. Set `isError: true` when any stage fails \
                            so the caller knows to iterate. Output is a structured report per stage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": {
                        "type": "string",
                        "description": "Project name (from list-projects) OR absolute path."
                    },
                    "stages": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["typecheck", "lint", "build", "test"] },
                        "description": "Optional subset of stages to run. Default: typecheck,lint,build."
                    }
                },
                "required": ["project"]
            }
        },
        {
            "name": "run-tests",
            "description": "Run the project's test suite (cargo test / npm test / pytest). \
                            Blocking — returns isError=true if any test fails.",
            "inputSchema": {
                "type": "object",
                "properties": { "project": { "type": "string" } },
                "required": ["project"]
            }
        },
        {
            "name": "logs",
            "description": "Read the last N lines of a log file (default 200). \
                            Accepts absolute path or a path relative to a project directory \
                            (if `project` is given). Useful for inspecting dev-server logs, \
                            Next.js build output, Payload logs, etc.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "path":    { "type": "string", "description": "Log file path." },
                    "lines":   { "type": "integer", "description": "Default 200.", "minimum": 1, "maximum": 5000 }
                },
                "required": ["path"]
            }
        },
        {
            "name": "reference-set",
            "description": "Archive an image as the visual reference (\"so debe verse\") for a project + view name. \
                            The reference is copied into the evidence store and used by visual-diff. \
                            Source can be a local file path the user just saved (e.g. ~/Desktop/admin-ref.png).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Project name from list-projects." },
                    "view":    { "type": "string", "description": "Logical view name, e.g. 'admin-dashboard', 'home-mobile'." },
                    "source_path": { "type": "string", "description": "Absolute path to the image file to archive." }
                },
                "required": ["project", "view", "source_path"]
            }
        },
        {
            "name": "screenshot",
            "description": "Capture a URL with headless Chromium (via npx playwright) and save as PNG. \
                            Output goes to the evidence store as the latest 'actual' for that view.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url":     { "type": "string", "description": "Full URL to capture (https://...)." },
                    "project": { "type": "string", "description": "Project name (used for evidence path)." },
                    "view":    { "type": "string", "description": "Logical view name (matches reference-set)." },
                    "viewport_width":  { "type": "integer", "description": "Default 1280." },
                    "viewport_height": { "type": "integer", "description": "Default 1080." },
                    "full_page": { "type": "boolean", "description": "Capture full scrollable page. Default false." },
                    "wait_ms":  { "type": "integer", "description": "Extra wait before capture, in ms. Default 1500." }
                },
                "required": ["url", "project", "view"]
            }
        },
        {
            "name": "visual-diff",
            "description": "Compare the stored reference for project+view against the latest actual capture (or an explicit pair). \
                            Returns isError=true when diff exceeds tolerance. Always writes a diff.png highlighting changed pixels.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "view":    { "type": "string" },
                    "reference_path": { "type": "string", "description": "Override stored reference." },
                    "actual_path":    { "type": "string", "description": "Override stored actual." },
                    "tolerance_percent": { "type": "number", "description": "Acceptable diff %. Default 0.5." }
                },
                "required": ["project", "view"]
            }
        },
        {
            "name": "lessons-log",
            "description": "Append a recurring-error lesson to the workspace KB so future iterations can search it. \
                            Use this after fixing a non-obvious bug — record the symptom and the fix.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project":  { "type": "string" },
                    "category": { "type": "string", "description": "e.g. 'css', 'build', 'deploy', 'auth', 'visual'." },
                    "symptom":  { "type": "string", "description": "What broke (one sentence)." },
                    "fix":      { "type": "string", "description": "What resolved it." }
                },
                "required": ["category", "symptom", "fix"]
            }
        },
        {
            "name": "lessons-search",
            "description": "Search the lessons KB for prior fixes matching a substring. Case-insensitive. \
                            Call this BEFORE attempting a fix to avoid re-discovering past solutions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":   { "type": "string" },
                    "project": { "type": "string", "description": "Filter to one project (optional)." }
                },
                "required": ["query"]
            }
        },
        {
            "name": "railway-logs",
            "description": "Tail Railway deployment logs for a project. Requires the `railway` CLI \
                            to be installed and logged in. The project must have `.railway` or \
                            the call must include `service`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "service": { "type": "string", "description": "Railway service name (optional)." },
                    "lines":   { "type": "integer", "description": "Default 200." }
                },
                "required": ["project"]
            }
        },
        {
            "name": "session.start",
            "description": "FAT TOOL: Load complete project context in one call. Returns contract, QA state, recent lessons, last commit, next task, and antipatterns.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Project name (e.g. 'puebloladehesa-rediseno')" }
                },
                "required": ["project"]
            }
        },
        {
            "name": "session.report",
            "description": "Generate structured markdown report of current project state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" }
                },
                "required": ["project"]
            }
        },
        {
            "name": "iterate.section",
            "description": "FAT TOOL: Autonomous loop — screenshot → diff → fix → commit → push until converged or stagnated. Returns iteration count, final diff, and commits.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": "Project name" },
                    "section": { "type": "string", "description": "Section ID to iterate (e.g. 'casas-grid')" },
                    "deploy_url": { "type": "string", "description": "Deploy URL for screenshots" },
                    "max_iterations": { "type": "integer", "description": "Default 999" },
                    "tolerance_percent": { "type": "number", "description": "Visual diff tolerance. Default 2.0" }
                },
                "required": ["project", "section", "deploy_url"]
            }
        },
        {
            "name": "contract.validate-action",
            "description": "Validate action against project contract before execution. Returns allow=false if out-of-scope.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "type": { "type": "string", "enum": ["edit", "write", "commit", "push", "deploy"] },
                    "target": { "type": "string", "description": "File or section affected" },
                    "details": { "type": "string", "description": "Action description" }
                },
                "required": ["type"]
            }
        },
        {
            "name": "contract.propose-change",
            "description": "Queue out-of-scope change proposal for human approval.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "reason": { "type": "string", "description": "Why the change is needed" },
                    "diff": { "type": "string", "description": "What would be added/changed" }
                },
                "required": ["reason"]
            }
        }
    ])
}

// ─── Dispatch ───────────────────────────────────────────────────────────────

fn handle_tool_call(id: Value, params: Option<&Value>) -> Value {
    let params = match params {
        Some(p) => p,
        None => return error_response(id, -32602, "Missing params"),
    };
    let tool = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let workspace_root = workspace_root();
    
    match tool {
        "list-projects" => ok_json(id, list_projects_tool()),
        "validate" => validate_tool(id, &args),
        "run-tests" => run_tests_tool(id, &args),
        "logs" => logs_tool(id, &args),
        "railway-logs" => railway_logs_tool(id, &args),
        "reference-set" => reference_set_tool(id, &args),
        "screenshot" => screenshot_tool(id, &args),
        "visual-diff" => visual_diff_tool(id, &args),
        "lessons-log" => lessons_log_tool(id, &args),
        "lessons-search" => lessons_search_tool(id, &args),
        
        // FAT TOOLS — Autonomous supervisor
        "session.start" => tools::session::session_start_tool(id, &args, &workspace_root),
        "session.report" => tools::session::session_report_tool(id, &args, &workspace_root),
        "iterate.section" => tools::iterate::iterate_section_tool(id, &args),
        "contract.validate-action" => tools::contract::validate_action_tool(id, &args),
        "contract.propose-change" => tools::contract::propose_change_tool(id, &args),
        
        _ => tool_error(id, &format!("Unknown tool: {}", tool)),
    }
}

// ─── Visual evidence helpers ────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn view_dir(project: &str, view: &str) -> PathBuf {
    evidence_dir()
        .join(slugify(project))
        .join(slugify(view))
}

fn reference_path(project: &str, view: &str) -> PathBuf {
    view_dir(project, view).join("reference.png")
}
fn actual_path(project: &str, view: &str) -> PathBuf {
    view_dir(project, view).join("actual.png")
}
fn diff_path(project: &str, view: &str) -> PathBuf {
    view_dir(project, view).join("diff.png")
}

// ─── Tool: reference-set ────────────────────────────────────────────────────

fn reference_set_tool(id: Value, args: &Value) -> Value {
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let view = match args.get("view").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: view"),
    };
    let source = match args.get("source_path").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: source_path"),
    };

    let src = PathBuf::from(source);
    if !src.is_file() {
        return tool_error(id, &format!("Source file not found: {}", src.display()));
    }
    let dir = view_dir(project, view);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return tool_error(id, &format!("mkdir {}: {}", dir.display(), e));
    }
    let dst = reference_path(project, view);
    if let Err(e) = std::fs::copy(&src, &dst) {
        return tool_error(id, &format!("copy failed: {}", e));
    }
    ok_text(
        id,
        &format!(
            "Reference archived for {}::{}\n  source: {}\n  stored: {}",
            project,
            view,
            src.display(),
            dst.display()
        ),
        false,
    )
}

// ─── Tool: screenshot ───────────────────────────────────────────────────────

fn screenshot_tool(id: Value, args: &Value) -> Value {
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: url"),
    };
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let view = match args.get("view").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: view"),
    };
    let vw = args.get("viewport_width").and_then(|v| v.as_u64()).unwrap_or(1280);
    let vh = args.get("viewport_height").and_then(|v| v.as_u64()).unwrap_or(1080);
    let full = args.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false);
    let wait_ms = args.get("wait_ms").and_then(|v| v.as_u64()).unwrap_or(1500);

    let dir = view_dir(project, view);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return tool_error(id, &format!("mkdir: {}", e));
    }
    let out = actual_path(project, view);

    // Use `npx playwright screenshot` — bundled with playwright global.
    let mut cmd_args: Vec<String> = vec![
        "playwright".into(),
        "screenshot".into(),
        format!("--viewport-size={},{}", vw, vh),
        format!("--wait-for-timeout={}", wait_ms),
    ];
    if full {
        cmd_args.push("--full-page".into());
    }
    cmd_args.push(url.into());
    cmd_args.push(out.to_string_lossy().into_owned());

    let res = Command::new("npx").args(&cmd_args).output();
    match res {
        Ok(o) if o.status.success() && out.is_file() => {
            let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
            ok_text(
                id,
                &format!(
                    "Screenshot captured: {} ({} bytes)\n  url: {}\n  viewport: {}x{}{}",
                    out.display(),
                    bytes,
                    url,
                    vw,
                    vh,
                    if full { " (full-page)" } else { "" }
                ),
                false,
            )
        }
        Ok(o) => tool_error(
            id,
            &format!(
                "playwright screenshot failed (exit={}):\nstdout: {}\nstderr: {}",
                o.status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
        ),
        Err(e) => tool_error(
            id,
            &format!("Failed to invoke npx playwright: {}. Ensure playwright is installed.", e),
        ),
    }
}

// ─── Tool: visual-diff ──────────────────────────────────────────────────────

fn visual_diff_tool(id: Value, args: &Value) -> Value {
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let view = match args.get("view").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: view"),
    };
    let tolerance = args
        .get("tolerance_percent")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let ref_p = args
        .get("reference_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| reference_path(project, view));
    let actual_p = args
        .get("actual_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| actual_path(project, view));
    let diff_p = diff_path(project, view);

    if !ref_p.is_file() {
        return tool_error(
            id,
            &format!(
                "No reference at {}. Set one first via reference-set.",
                ref_p.display()
            ),
        );
    }
    if !actual_p.is_file() {
        return tool_error(
            id,
            &format!(
                "No actual capture at {}. Run screenshot first.",
                actual_p.display()
            ),
        );
    }

    let script = visual_diff_script();
    if !script.is_file() {
        return tool_error(id, &format!("Diff script missing: {}", script.display()));
    }

    let out = Command::new("node")
        .arg(&script)
        .arg(&ref_p)
        .arg(&actual_p)
        .arg(&diff_p)
        .arg(format!("{}", tolerance))
        .output();

    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            // The script emits a single JSON line on stdout.
            let parsed: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
            let passed = parsed
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let percent = parsed
                .get("diff_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(-1.0);
            let reason = parsed
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let msg = format!(
                "Visual diff for {}::{}\n  reference: {}\n  actual:    {}\n  diff:      {}\n  tolerance: {:.2}%\n  measured:  {:.4}%\n  status:    {}{}",
                project,
                view,
                ref_p.display(),
                actual_p.display(),
                diff_p.display(),
                tolerance,
                percent,
                if passed { "PASS" } else { "BLOCK — visual regression" },
                if reason.is_empty() { String::new() } else { format!(" ({})", reason) }
            );
            let mut text = msg;
            if !stderr.is_empty() {
                text.push_str("\n\n[diff stderr]\n");
                text.push_str(&stderr);
            }
            ok_text(id, &text, !passed)
        }
        Err(e) => tool_error(id, &format!("Failed to run diff script: {}", e)),
    }
}

// ─── Tool: lessons-log / lessons-search ─────────────────────────────────────

fn lessons_log_tool(id: Value, args: &Value) -> Value {
    let category = match args.get("category").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: category"),
    };
    let symptom = match args.get("symptom").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: symptom"),
    };
    let fix = match args.get("fix").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: fix"),
    };
    let project = args.get("project").and_then(|v| v.as_str()).unwrap_or("");

    let entry = json!({
        "ts": chrono_like_now(),
        "project": project,
        "category": category,
        "symptom": symptom,
        "fix": fix,
    });
    let line = serde_json::to_string(&entry).unwrap();

    let path = lessons_file();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{}", line) {
                return tool_error(id, &format!("write failed: {}", e));
            }
            ok_text(id, &format!("Logged lesson to {}\n  {}", path.display(), line), false)
        }
        Err(e) => tool_error(id, &format!("open lessons file: {}", e)),
    }
}

fn lessons_search_tool(id: Value, args: &Value) -> Value {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: query"),
    };
    let project_filter = args.get("project").and_then(|v| v.as_str());
    let q = query.to_lowercase();

    let path = lessons_file();
    if !path.is_file() {
        return ok_text(id, "No lessons recorded yet.", false);
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return tool_error(id, &format!("read failed: {}", e)),
    };
    let mut hits: Vec<Value> = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(pf) = project_filter {
            if v.get("project").and_then(|x| x.as_str()).unwrap_or("") != pf {
                continue;
            }
        }
        let blob = format!(
            "{} {} {} {}",
            v.get("category").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("symptom").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("fix").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("project").and_then(|x| x.as_str()).unwrap_or(""),
        )
        .to_lowercase();
        if blob.contains(&q) {
            hits.push(v);
        }
    }

    let summary = if hits.is_empty() {
        format!("No lessons matched query: '{}'", query)
    } else {
        let mut s = format!("{} lesson(s) matched '{}':\n\n", hits.len(), query);
        for h in &hits {
            s.push_str(&format!(
                "[{}] {} ({})\n  symptom: {}\n  fix:     {}\n\n",
                h.get("ts").and_then(|x| x.as_str()).unwrap_or(""),
                h.get("category").and_then(|x| x.as_str()).unwrap_or(""),
                h.get("project").and_then(|x| x.as_str()).unwrap_or("global"),
                h.get("symptom").and_then(|x| x.as_str()).unwrap_or(""),
                h.get("fix").and_then(|x| x.as_str()).unwrap_or(""),
            ));
        }
        s
    };
    ok_text(id, &summary, false)
}

/// Cheap RFC3339-ish timestamp without pulling chrono.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // ISO-like: just emit the unix epoch — easy to grep, easy to sort.
    format!("ts={}", secs)
}

// ─── Workspace & project resolution ─────────────────────────────────────────

fn workspace_root() -> PathBuf {
    if let Ok(v) = std::env::var("RETARGET_WORKSPACE") {
        return PathBuf::from(v);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join("Desktop/RETARGET-WORKSPACE/PROJECTS");
    }
    PathBuf::from(".")
}

#[derive(Debug)]
struct Project {
    name: String,
    path: PathBuf,
    kind: &'static str,
}

fn detect_kind(path: &Path) -> Option<&'static str> {
    let has = |f: &str| path.join(f).exists();
    if has("next.config.js") || has("next.config.ts") || has("next.config.mjs") {
        Some("next")
    } else if has("package.json") {
        Some("node")
    } else if has("Cargo.toml") {
        Some("rust")
    } else if has("pyproject.toml") || has("requirements.txt") {
        Some("python")
    } else {
        None
    }
}

fn discover_projects() -> Vec<Project> {
    let root = workspace_root();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        // Check project itself
        if let Some(kind) = detect_kind(&p) {
            out.push(Project {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: p.clone(),
                kind,
            });
            continue;
        }
        // One level deeper (e.g. monorepo: PROJECTS/news-cms/news-ai-cms)
        if let Ok(children) = std::fs::read_dir(&p) {
            for child in children.flatten() {
                let cp = child.path();
                if !cp.is_dir() {
                    continue;
                }
                if let Some(kind) = detect_kind(&cp) {
                    let parent = entry.file_name().to_string_lossy().into_owned();
                    let name = child.file_name().to_string_lossy().into_owned();
                    out.push(Project {
                        name: format!("{}/{}", parent, name),
                        path: cp,
                        kind,
                    });
                }
            }
        }
    }
    out
}

fn resolve_project(spec: &str) -> Result<PathBuf, String> {
    let p = Path::new(spec);
    if p.is_absolute() && p.is_dir() {
        return Ok(p.to_path_buf());
    }
    let projects = discover_projects();
    for pr in &projects {
        if pr.name == spec || pr.name.ends_with(&format!("/{}", spec)) {
            return Ok(pr.path.clone());
        }
    }
    Err(format!(
        "Project not found: '{}'. Run list-projects to see available names.",
        spec
    ))
}

// ─── Tool: list-projects ────────────────────────────────────────────────────

fn list_projects_tool() -> Value {
    let projects = discover_projects();
    let root = workspace_root();
    let arr: Vec<Value> = projects
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "path": p.path.to_string_lossy(),
                "kind": p.kind,
            })
        })
        .collect();
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "Workspace: {}\nFound {} project(s):\n{}",
                root.display(),
                arr.len(),
                serde_json::to_string_pretty(&arr).unwrap_or_default()
            )
        }],
        "isError": false
    })
}

// ─── Tool: validate ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct StageResult {
    name: String,
    passed: bool,
    skipped: bool,
    duration_ms: u128,
    output: String,
}

fn validate_tool(id: Value, args: &Value) -> Value {
    let project_spec = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let path = match resolve_project(project_spec) {
        Ok(p) => p,
        Err(e) => return tool_error(id, &e),
    };

    let requested: Vec<String> = args
        .get("stages")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(|| vec!["typecheck".into(), "lint".into(), "build".into()]);

    let kind = detect_kind(&path).unwrap_or("unknown");
    let mut results: Vec<StageResult> = Vec::new();
    let mut fail_fast = false;

    for stage in &requested {
        if fail_fast {
            results.push(StageResult {
                name: stage.clone(),
                passed: false,
                skipped: true,
                duration_ms: 0,
                output: "(skipped: previous stage failed)".into(),
            });
            continue;
        }
        let res = run_stage(&path, kind, stage);
        if !res.passed && !res.skipped {
            fail_fast = true;
        }
        results.push(res);
    }

    let all_passed = results.iter().all(|r| r.passed || r.skipped);
    let report = render_report(&path, kind, &results);
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": !all_passed,
            "content": [{ "type": "text", "text": report }]
        }
    })
}

fn run_stage(path: &Path, kind: &str, stage: &str) -> StageResult {
    let start = Instant::now();
    let cmd_opt = match (kind, stage) {
        ("next" | "node", "typecheck") => Some(("npx", vec!["--no-install", "tsc", "--noEmit"])),
        ("next" | "node", "lint") => Some(("npm", vec!["run", "--silent", "lint"])),
        ("next", "build") => Some(("npm", vec!["run", "--silent", "build"])),
        ("node", "build") => Some(("npm", vec!["run", "--silent", "build"])),
        ("next" | "node", "test") => Some(("npm", vec!["test", "--silent"])),

        ("rust", "typecheck") => Some(("cargo", vec!["check", "--quiet"])),
        ("rust", "lint") => Some(("cargo", vec!["clippy", "--quiet", "--", "-D", "warnings"])),
        ("rust", "build") => Some(("cargo", vec!["build", "--release", "--quiet"])),
        ("rust", "test") => Some(("cargo", vec!["test", "--quiet"])),

        ("python", "typecheck") => Some(("python3", vec!["-m", "compileall", "-q", "."])),
        ("python", "lint") => Some(("python3", vec!["-m", "ruff", "check", "."])),
        ("python", "build") => None, // no-op
        ("python", "test") => Some(("python3", vec!["-m", "pytest", "-q"])),

        _ => None,
    };

    let Some((cmd, args)) = cmd_opt else {
        return StageResult {
            name: stage.into(),
            passed: true,
            skipped: true,
            duration_ms: start.elapsed().as_millis(),
            output: format!("(not applicable for kind='{}')", kind),
        };
    };

    let out = Command::new(cmd).args(&args).current_dir(path).output();
    let duration_ms = start.elapsed().as_millis();
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let combined = format!(
                "$ {} {}\n--- stdout ---\n{}\n--- stderr ---\n{}\nexit={}",
                cmd,
                args.join(" "),
                stdout.trim_end(),
                stderr.trim_end(),
                o.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
            );
            StageResult {
                name: stage.into(),
                passed: o.status.success(),
                skipped: false,
                duration_ms,
                output: combined,
            }
        }
        Err(e) => StageResult {
            name: stage.into(),
            passed: false,
            skipped: false,
            duration_ms,
            output: format!("spawn failed: {}: {}", cmd, e),
        },
    }
}

fn render_report(path: &Path, kind: &str, stages: &[StageResult]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Quality Gate — {} ({})\n{}\n",
        path.display(),
        kind,
        "═".repeat(60)
    ));
    for s in stages {
        let icon = if s.skipped {
            "○"
        } else if s.passed {
            "✓"
        } else {
            "✗"
        };
        out.push_str(&format!(
            "\n{} {:<10} ({} ms){}\n",
            icon,
            s.name,
            s.duration_ms,
            if s.skipped { " [skipped]" } else { "" }
        ));
        if !s.passed || !s.skipped {
            // Include output for failed OR non-skipped stages; trim to keep payload tight.
            let truncated = truncate(&s.output, 4000);
            out.push_str(&truncated);
            if !truncated.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    let passed_count = stages.iter().filter(|s| s.passed && !s.skipped).count();
    let failed_count = stages.iter().filter(|s| !s.passed && !s.skipped).count();
    let skipped_count = stages.iter().filter(|s| s.skipped).count();
    out.push_str(&format!(
        "\n{}\nSummary: {} passed, {} failed, {} skipped\n",
        "═".repeat(60),
        passed_count,
        failed_count,
        skipped_count
    ));
    if failed_count > 0 {
        out.push_str("Status: BLOCKED — fix the failing stage(s) and re-run.\n");
    } else {
        out.push_str("Status: OK\n");
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Keep head + tail; middle is where most noise lives in compiler output.
    let head = &s[..max / 2];
    let tail = &s[s.len() - max / 2..];
    format!(
        "{}\n... [truncated {} bytes] ...\n{}",
        head,
        s.len() - max,
        tail
    )
}

// ─── Tool: run-tests ────────────────────────────────────────────────────────

fn run_tests_tool(id: Value, args: &Value) -> Value {
    let spec = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let path = match resolve_project(spec) {
        Ok(p) => p,
        Err(e) => return tool_error(id, &e),
    };
    let kind = detect_kind(&path).unwrap_or("unknown");
    let res = run_stage(&path, kind, "test");
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": !res.passed,
            "content": [{ "type": "text", "text": res.output }]
        }
    })
}

// ─── Tool: logs ─────────────────────────────────────────────────────────────

fn logs_tool(id: Value, args: &Value) -> Value {
    let log_path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return tool_error(id, "Missing argument: path"),
    };
    let n = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

    let resolved = if Path::new(log_path).is_absolute() {
        PathBuf::from(log_path)
    } else if let Some(proj) = args.get("project").and_then(|v| v.as_str()) {
        match resolve_project(proj) {
            Ok(p) => p.join(log_path),
            Err(e) => return tool_error(id, &e),
        }
    } else {
        PathBuf::from(log_path)
    };

    if !resolved.is_file() {
        return tool_error(id, &format!("Log file not found: {}", resolved.display()));
    }

    match std::fs::read_to_string(&resolved) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(n);
            let tail = lines[start..].join("\n");
            let header = format!(
                "── {} (last {} of {} lines) ──\n",
                resolved.display(),
                lines.len() - start,
                lines.len()
            );
            ok_text(id, &format!("{}{}", header, tail), false)
        }
        Err(e) => tool_error(id, &format!("Read failed: {}", e)),
    }
}

// ─── Tool: railway-logs ─────────────────────────────────────────────────────

fn railway_logs_tool(id: Value, args: &Value) -> Value {
    let spec = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    let path = match resolve_project(spec) {
        Ok(p) => p,
        Err(e) => return tool_error(id, &e),
    };
    let n = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

    let mut cmd_args: Vec<String> = vec!["logs".into()];
    if let Some(svc) = args.get("service").and_then(|v| v.as_str()) {
        cmd_args.push("-s".into());
        cmd_args.push(svc.into());
    }

    let out = Command::new("railway")
        .args(&cmd_args)
        .current_dir(&path)
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            let start = lines.len().saturating_sub(n);
            let tail = lines[start..].join("\n");
            ok_text(id, &tail, false)
        }
        Ok(o) => tool_error(
            id,
            &format!(
                "railway logs failed (exit={}):\n{}",
                o.status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
                String::from_utf8_lossy(&o.stderr)
            ),
        ),
        Err(e) => tool_error(
            id,
            &format!("railway CLI not available: {}. Install with: npm i -g @railway/cli", e),
        ),
    }
}

// ─── Response helpers ───────────────────────────────────────────────────────

fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn tool_error(id: Value, msg: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": true,
            "content": [{ "type": "text", "text": msg }]
        }
    })
}

fn ok_text(id: Value, text: &str, is_error: bool) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": is_error,
            "content": [{ "type": "text", "text": text }]
        }
    })
}

fn ok_json(id: Value, inner: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": inner })
}

async fn write_line(stdout: &mut io::Stdout, value: &Value) -> io::Result<()> {
    let mut line = serde_json::to_string(value).unwrap();
    line.push('\n');
    stdout.write_all(line.as_bytes()).await?;
    stdout.flush().await?;
    Ok(())
}
