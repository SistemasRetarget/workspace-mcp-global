//! Session management tools
//! 
//! Fat tools que cargan TODO el contexto del proyecto en una sola llamada.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Carga el contrato del proyecto desde contracts/<project>.json
fn load_contract(project: &str) -> Result<Value, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let contract_path = PathBuf::from(home)
        .join("Documents/workspace-mcp-global/contracts")
        .join(format!("{}.json", project));
    
    if !contract_path.is_file() {
        return Err(format!("No contract found for project '{}' at {}", project, contract_path.display()));
    }
    
    let content = fs::read_to_string(&contract_path)
        .map_err(|e| format!("Failed to read contract: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid contract JSON: {}", e))
}

/// Carga el estado QA del proyecto desde .qa-state.json
fn load_qa_state(project_path: &Path) -> Value {
    let state_path = project_path.join(".qa-state.json");
    if !state_path.is_file() {
        return json!({
            "status": "no state file found",
            "sections": []
        });
    }
    
    match fs::read_to_string(&state_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(json!({})),
        Err(_) => json!({"status": "failed to read state"})
    }
}

/// Busca las últimas N lecciones relevantes del proyecto
fn load_recent_lessons(project: &str, limit: usize) -> Vec<Value> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let lessons_path = PathBuf::from(home)
        .join("Documents/workspace-mcp-global/lessons/lessons.jsonl");
    
    if !lessons_path.is_file() {
        return vec![];
    }
    
    let content = match fs::read_to_string(&lessons_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    
    let mut lessons: Vec<Value> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|v: &Value| {
            v.get("project")
                .and_then(|p| p.as_str())
                .map(|p| p == project || p.is_empty())
                .unwrap_or(false)
        })
        .collect();
    
    // Más recientes primero (asumiendo que el archivo se escribe en orden)
    lessons.reverse();
    lessons.truncate(limit);
    lessons
}

/// Obtiene el último commit del proyecto
fn get_last_commit(project_path: &Path) -> String {
    use std::process::Command;
    
    let output = Command::new("git")
        .args(&["log", "-1", "--oneline"])
        .current_dir(project_path)
        .output();
    
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => "unknown".to_string()
    }
}

/// Determina la siguiente tarea basada en el contrato
fn get_next_task(contract: &Value) -> String {
    let sections = match contract.get("sections").and_then(|s| s.as_array()) {
        Some(s) => s,
        None => return "No sections defined in contract".to_string(),
    };
    
    // Buscar primera sección PENDING
    for section in sections {
        let status = section.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status == "PENDING" {
            let id = section.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
            return format!("iterate section: {}", id);
        }
    }
    
    "All sections complete — ready for final audit".to_string()
}

/// Carga antipatrones aplicables del proyecto
fn load_antipatterns(project_path: &Path) -> Vec<String> {
    let antipatterns_path = project_path.join("ANTIPATTERNS.md");
    if !antipatterns_path.is_file() {
        return vec![];
    }
    
    match fs::read_to_string(&antipatterns_path) {
        Ok(content) => {
            // Extraer líneas que empiezan con "- " o "* "
            content
                .lines()
                .filter(|line| line.trim_start().starts_with("- ") || line.trim_start().starts_with("* "))
                .map(|line| line.trim().to_string())
                .take(10) // Limitar a 10 más relevantes
                .collect()
        }
        Err(_) => vec![]
    }
}

/// Tool: session.start
/// Carga TODO el contexto del proyecto en una sola llamada
pub fn session_start_tool(id: Value, args: &Value, workspace_root: &Path) -> Value {
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    
    // Cargar contrato
    let contract = match load_contract(project) {
        Ok(c) => c,
        Err(e) => return tool_error(id, &e),
    };
    
    // Resolver path del proyecto
    let project_path = workspace_root.join(project);
    if !project_path.is_dir() {
        return tool_error(id, &format!("Project directory not found: {}", project_path.display()));
    }
    
    // Cargar estado QA
    let qa_state = load_qa_state(&project_path);
    
    // Cargar lecciones recientes
    let lessons = load_recent_lessons(project, 5);
    
    // Obtener último commit
    let last_commit = get_last_commit(&project_path);
    
    // Determinar siguiente tarea
    let next_task = get_next_task(&contract);
    
    // Cargar antipatrones
    let antipatterns = load_antipatterns(&project_path);
    
    // Construir respuesta completa
    let response = json!({
        "project": project,
        "contract": contract,
        "qa_state": qa_state,
        "last_commit": last_commit,
        "lessons": lessons,
        "next_task": next_task,
        "antipatterns": antipatterns,
        "loaded_at": chrono_like_now()
    });
    
    let summary = format!(
        "Session started for project: {}\n\
        Contract version: {}\n\
        Last commit: {}\n\
        Sections DONE: {}\n\
        Sections PENDING: {}\n\
        Recent lessons: {}\n\
        Next task: {}\n\
        Antipatterns loaded: {}\n\n\
        Full context loaded. Ready to execute.",
        project,
        contract.get("version").and_then(|v| v.as_u64()).unwrap_or(0),
        last_commit,
        count_sections_by_status(&contract, "DONE"),
        count_sections_by_status(&contract, "PENDING"),
        lessons.len(),
        next_task,
        antipatterns.len()
    );
    
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": false,
            "content": [
                { "type": "text", "text": summary },
                { "type": "resource", "resource": {
                    "uri": format!("context://{}", project),
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&response).unwrap()
                }}
            ]
        }
    })
}

/// Tool: session.report
/// Genera reporte estructurado del estado actual
pub fn session_report_tool(id: Value, args: &Value, workspace_root: &Path) -> Value {
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: project"),
    };
    
    let contract = match load_contract(project) {
        Ok(c) => c,
        Err(e) => return tool_error(id, &e),
    };
    
    let project_path = workspace_root.join(project);
    let qa_state = load_qa_state(&project_path);
    let last_commit = get_last_commit(&project_path);
    
    // Generar reporte markdown
    let report = generate_markdown_report(project, &contract, &qa_state, &last_commit);
    
    ok_text(id, &report, false)
}

fn generate_markdown_report(project: &str, contract: &Value, _qa_state: &Value, last_commit: &str) -> String {
    let mut md = String::new();
    
    md.push_str(&format!("# Reporte de Progreso: {}\n\n", project));
    md.push_str(&format!("**Fecha:** {}\n", chrono_like_now()));
    md.push_str(&format!("**Último commit:** `{}`\n\n", last_commit));
    
    md.push_str("## Estado de Secciones\n\n");
    md.push_str("| Sección | Estado | Notas |\n");
    md.push_str("|---------|--------|-------|\n");
    
    if let Some(sections) = contract.get("sections").and_then(|s| s.as_array()) {
        for section in sections {
            let id = section.get("id").and_then(|i| i.as_str()).unwrap_or("?");
            let status = section.get("status").and_then(|s| s.as_str()).unwrap_or("?");
            let notes = section.get("notes").and_then(|n| n.as_str()).unwrap_or("");
            
            let icon = match status {
                "DONE" => "✅",
                "PENDING" => "⏸️",
                "BLOCKED" => "🚫",
                _ => "❓"
            };
            
            md.push_str(&format!("| {} {} | {} | {} |\n", icon, id, status, notes));
        }
    }
    
    md.push_str("\n## Métricas\n\n");
    md.push_str(&format!("- **Secciones completadas:** {}\n", count_sections_by_status(contract, "DONE")));
    md.push_str(&format!("- **Secciones pendientes:** {}\n", count_sections_by_status(contract, "PENDING")));
    md.push_str(&format!("- **Secciones bloqueadas:** {}\n", count_sections_by_status(contract, "BLOCKED")));
    
    if let Some(current) = contract.get("current_state") {
        if let Some(iterations) = current.get("total_iterations").and_then(|i| i.as_u64()) {
            md.push_str(&format!("- **Iteraciones totales:** {}\n", iterations));
        }
        if let Some(lessons) = current.get("lessons_logged").and_then(|l| l.as_u64()) {
            md.push_str(&format!("- **Lecciones registradas:** {}\n", lessons));
        }
    }
    
    md.push_str("\n## Próximos Pasos\n\n");
    md.push_str(&format!("- {}\n", get_next_task(contract)));
    
    md
}

fn count_sections_by_status(contract: &Value, status: &str) -> usize {
    contract
        .get("sections")
        .and_then(|s| s.as_array())
        .map(|sections| {
            sections
                .iter()
                .filter(|s| s.get("status").and_then(|st| st.as_str()) == Some(status))
                .count()
        })
        .unwrap_or(0)
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    // Formato ISO-like simple
    let dt = chrono::NaiveDateTime::from_timestamp_opt(secs as i64, 0)
        .unwrap_or_else(|| chrono::NaiveDateTime::from_timestamp(0, 0));
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
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

// Módulo chrono simplificado (sin dependencia externa)
mod chrono {
    pub struct NaiveDateTime {
        timestamp: i64,
    }
    
    impl NaiveDateTime {
        pub fn from_timestamp_opt(secs: i64, _nsecs: u32) -> Option<Self> {
            Some(Self { timestamp: secs })
        }
        
        pub fn from_timestamp(secs: i64, _nsecs: u32) -> Self {
            Self { timestamp: secs }
        }
        
        pub fn format(&self, _fmt: &str) -> FormattedDateTime {
            FormattedDateTime { timestamp: self.timestamp }
        }
    }
    
    pub struct FormattedDateTime {
        timestamp: i64,
    }
    
    impl std::fmt::Display for FormattedDateTime {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            // Formato simple YYYY-MM-DD HH:MM:SS UTC
            write!(f, "ts={}", self.timestamp)
        }
    }
}
