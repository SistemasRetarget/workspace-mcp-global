//! WordPress fat tools
//!
//! Provides direct REST API access to WP sites managed by Retarget:
//!   - wp_redirect_list     List active redirects (Redirection plugin)
//!   - wp_redirect_disable  Disable a redirect by ID
//!   - wp_redirect_create   Create a 301 redirect (slug → destination)
//!   - wp_cache_purge       Purge site cache (W3TC or Kinsta)

use serde_json::{json, Value};
use std::process::Command;

// ─── Site registry ────────────────────────────────────────────────────────────

struct WpSite {
    base_url: &'static str,
    user: &'static str,
    env_pass_key: &'static str,
}

fn site_config(site: &str) -> Option<WpSite> {
    match site {
        "puyehue" | "puyehue.cl" => Some(WpSite {
            base_url: "https://puyehue.cl",
            user: "hectorluis.maldonado@retarget.cl",
            env_pass_key: "WP_PUYEHUE_PASS",
        }),
        "tac" | "termasaguascalientes" | "termasaguascalientes.cl" => Some(WpSite {
            base_url: "https://termasaguascalientes.cl",
            user: "hectorluis.maldonado@retarget.cl",
            env_pass_key: "WP_TAC_PASS",
        }),
        "futangue" | "parquefutangue" | "parquefutangue.com" => Some(WpSite {
            base_url: "https://parquefutangue.com",
            user: "hectorluis.maldonado@retarget.cl",
            env_pass_key: "WP_FUTANGUE_PASS",
        }),
        _ => None,
    }
}

/// Build Basic Auth header value: base64(user:pass)
fn basic_auth(user: &str, pass: &str) -> String {
    let credentials = format!("{}:{}", user, pass);
    // Use openssl or python3 to base64 encode since we have no external crate
    let out = Command::new("python3")
        .args(["-c", &format!("import base64; print(base64.b64encode(b'{}').decode())", credentials)])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => {
            // Fallback: use openssl
            let out2 = Command::new("sh")
                .args(["-c", &format!("printf '{}' | openssl base64", credentials)])
                .output();
            match out2 {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().replace('\n', "")
                }
                _ => String::new()
            }
        }
    }
}

/// Execute a curl request and return (http_code, body)
fn curl_request(
    method: &str,
    url: &str,
    auth_header: &str,
    body: Option<&str>,
) -> Result<(String, String), String> {
    let body_path = std::env::temp_dir().join(format!(
        "qg-wp-{}-{}.json",
        std::process::id(),
        url.len()
    ));
    let body_path_str = body_path.to_string_lossy().to_string();

    let mut args: Vec<String> = vec![
        "-sSL".into(),
        "-m".into(), "30".into(),
        "-X".into(), method.to_uppercase(),
        "-H".into(), format!("Authorization: Basic {}", auth_header),
        "-H".into(), "Content-Type: application/json".into(),
        "-o".into(), body_path_str.clone(),
        "-w".into(), "%{http_code}".into(),
    ];

    if let Some(b) = body {
        args.push("-d".into());
        args.push(b.to_string());
    }
    args.push(url.to_string());

    let out = Command::new("/usr/bin/curl")
        .args(&args)
        .output()
        .map_err(|e| format!("curl spawn error: {}", e))?;

    let http_code = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let body_content = std::fs::read_to_string(&body_path).unwrap_or_default();
    let _ = std::fs::remove_file(&body_path);

    if !out.status.success() && http_code.is_empty() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(format!("curl failed: {}", stderr));
    }

    Ok((http_code, body_content))
}

/// Resolve site config and app password, returning (WpSite, app_password) or error string
fn resolve_site(site: &str) -> Result<(WpSite, String), String> {
    let cfg = site_config(site)
        .ok_or_else(|| format!(
            "Unknown site '{}'. Valid: puyehue, tac, futangue", site
        ))?;

    let pass = std::env::var(cfg.env_pass_key).map_err(|_| {
        format!(
            "Environment variable {} not set. Export it before calling this tool.",
            cfg.env_pass_key
        )
    })?;

    Ok((cfg, pass))
}

// ─── Tool: wp_redirect_list ──────────────────────────────────────────────────

/// Lists all redirects registered in the Redirection plugin for a WP site.
pub fn wp_redirect_list_tool(id: Value, args: &Value) -> Value {
    let site = match args.get("site").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: site (puyehue | tac | futangue)"),
    };

    let (cfg, pass) = match resolve_site(site) {
        Ok(r) => r,
        Err(e) => return tool_error(id, &e),
    };

    let auth = basic_auth(cfg.user, &pass);
    let url = format!("{}/wp-json/redirection/v1/redirect?per_page=100", cfg.base_url);

    match curl_request("GET", &url, &auth, None) {
        Ok((code, body)) => {
            if code == "200" {
                let parsed: Value = serde_json::from_str(&body).unwrap_or(json!({"raw": body}));
                let summary = format_redirect_list(&parsed);
                tool_ok(id, &summary, &parsed)
            } else {
                tool_error(
                    id,
                    &format!("HTTP {} from {}:\n{}", code, url, truncate_body(&body, 800)),
                )
            }
        }
        Err(e) => tool_error(id, &e),
    }
}

fn format_redirect_list(data: &Value) -> String {
    // Redirection plugin returns { "items": [...], "total": N }
    let items = data
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if items.is_empty() {
        return "No redirects found.".to_string();
    }

    let mut out = format!("{} redirect(s) found:\n\n", items.len());
    for item in &items {
        let id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("?");
        let action_url = item
            .get("action_data")
            .and_then(|d| d.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let code = item.get("action_code").and_then(|v| v.as_i64()).unwrap_or(0);
        let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("?");
        out.push_str(&format!(
            "  ID={:<5} [{:>8}] {} → {} ({})\n",
            id, status, url, action_url, code
        ));
    }
    out
}

// ─── Tool: wp_redirect_disable ───────────────────────────────────────────────

/// Disables a redirect by ID using the Redirection plugin bulk endpoint.
pub fn wp_redirect_disable_tool(id: Value, args: &Value) -> Value {
    let site = match args.get("site").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: site"),
    };
    let redirect_id = match args.get("redirect_id").and_then(|v| v.as_i64()) {
        Some(n) => n,
        None => return tool_error(id, "Missing argument: redirect_id (integer)"),
    };

    let (cfg, pass) = match resolve_site(site) {
        Ok(r) => r,
        Err(e) => return tool_error(id, &e),
    };

    let auth = basic_auth(cfg.user, &pass);
    let url = format!("{}/wp-json/redirection/v1/bulk", cfg.base_url);

    let body = json!({
        "items": [redirect_id],
        "type": "redirect",
        "status": "disabled"
    });
    let body_str = serde_json::to_string(&body).unwrap();

    match curl_request("POST", &url, &auth, Some(&body_str)) {
        Ok((code, resp_body)) => {
            if code == "200" {
                let parsed: Value =
                    serde_json::from_str(&resp_body).unwrap_or(json!({"raw": resp_body}));
                tool_ok(
                    id,
                    &format!(
                        "Redirect ID={} disabled on {}. HTTP {}.",
                        redirect_id, site, code
                    ),
                    &parsed,
                )
            } else {
                tool_error(
                    id,
                    &format!(
                        "HTTP {} disabling redirect ID={} on {}:\n{}",
                        code,
                        redirect_id,
                        site,
                        truncate_body(&resp_body, 800)
                    ),
                )
            }
        }
        Err(e) => tool_error(id, &e),
    }
}

// ─── Tool: wp_redirect_create ────────────────────────────────────────────────

/// Creates a 301 redirect: slug → destination.
pub fn wp_redirect_create_tool(id: Value, args: &Value) -> Value {
    let site = match args.get("site").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: site"),
    };
    let slug = match args.get("slug").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: slug (e.g. /promo-verano/)"),
    };
    let destination = match args.get("destination").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: destination (full URL)"),
    };

    let (cfg, pass) = match resolve_site(site) {
        Ok(r) => r,
        Err(e) => return tool_error(id, &e),
    };

    let auth = basic_auth(cfg.user, &pass);
    let url = format!("{}/wp-json/redirection/v1/redirect", cfg.base_url);

    let body = json!({
        "url": slug,
        "action_type": "url",
        "action_code": 301,
        "action_data": { "url": destination }
    });
    let body_str = serde_json::to_string(&body).unwrap();

    match curl_request("POST", &url, &auth, Some(&body_str)) {
        Ok((code, resp_body)) => {
            if code == "200" || code == "201" {
                let parsed: Value =
                    serde_json::from_str(&resp_body).unwrap_or(json!({"raw": resp_body}));
                let new_id = parsed
                    .get("id")
                    .and_then(|v| v.as_i64())
                    .map(|n| format!(" (new ID={})", n))
                    .unwrap_or_default();
                tool_ok(
                    id,
                    &format!(
                        "Redirect created on {}: {} → {}{}. HTTP {}.",
                        site, slug, destination, new_id, code
                    ),
                    &parsed,
                )
            } else {
                tool_error(
                    id,
                    &format!(
                        "HTTP {} creating redirect on {}:\n{}",
                        code,
                        site,
                        truncate_body(&resp_body, 800)
                    ),
                )
            }
        }
        Err(e) => tool_error(id, &e),
    }
}

// ─── Tool: wp_cache_purge ────────────────────────────────────────────────────

/// Purges site cache via W3TC REST endpoint or Kinsta cache API.
/// Tries W3TC first (POST /wp-json/w3tc/v1/flush_all), then falls back to
/// a WP-CLI shell command if available.
pub fn wp_cache_purge_tool(id: Value, args: &Value) -> Value {
    let site = match args.get("site").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: site"),
    };

    let (cfg, pass) = match resolve_site(site) {
        Ok(r) => r,
        Err(e) => return tool_error(id, &e),
    };

    let auth = basic_auth(cfg.user, &pass);

    // Strategy 1: W3TC REST endpoint (if installed)
    let w3tc_url = format!("{}/wp-json/w3tc/v1/flush_all", cfg.base_url);
    match curl_request("POST", &w3tc_url, &auth, Some("{}")) {
        Ok((code, body)) if code == "200" || code == "201" => {
            let parsed: Value = serde_json::from_str(&body).unwrap_or(json!({"raw": body}));
            return tool_ok(
                id,
                &format!("Cache purged on {} via W3TC. HTTP {}.", site, code),
                &parsed,
            );
        }
        _ => {}
    }

    // Strategy 2: Kinsta cache purge (for Kinsta-hosted sites)
    // Kinsta uses a special endpoint: /?kinsta-cache-purge=full
    let kinsta_url = format!("{}/?kinsta-cache-purge=full", cfg.base_url);
    match curl_request("GET", &kinsta_url, &auth, None) {
        Ok((code, body)) if code == "200" => {
            return tool_ok(
                id,
                &format!("Cache purge request sent to {} (Kinsta). HTTP {}.", site, code),
                &json!({"strategy": "kinsta", "response_length": body.len()}),
            );
        }
        _ => {}
    }

    // Strategy 3: WP Super Cache or generic — call the WP REST cache endpoint
    let wp_cache_url = format!("{}/wp-json/wp/v2/settings", cfg.base_url);
    // As a last resort, report that no cache plugin was detected
    match curl_request("GET", &wp_cache_url, &auth, None) {
        Ok((code, _)) if code == "200" => {
            tool_error(
                id,
                &format!(
                    "WP REST API reachable on {} but no supported cache plugin detected. \
                     Install W3 Total Cache (W3TC) or use Kinsta dashboard to purge cache manually.",
                    site
                ),
            )
        }
        Ok((code, body)) => tool_error(
            id,
            &format!(
                "Cache purge failed on {}. HTTP {}. Response:\n{}",
                site,
                code,
                truncate_body(&body, 400)
            ),
        ),
        Err(e) => tool_error(id, &format!("Cache purge error on {}: {}", site, e)),
    }
}

// ─── Response helpers ─────────────────────────────────────────────────────────

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

fn tool_ok(id: Value, summary: &str, data: &Value) -> Value {
    let text = format!(
        "{}\n\nData:\n{}",
        summary,
        serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string())
    );
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": false,
            "content": [{ "type": "text", "text": text }]
        }
    })
}

fn truncate_body(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}… [truncated {} bytes]", &s[..max], s.len() - max)
    }
}
