//! Contract enforcement tools
//!
//! Valida acciones contra el contrato del proyecto antes de ejecutarse

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

pub fn validate_action_tool(id: Value, args: &Value) -> Value {
    let action_type = match args.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return tool_error(id, "Missing argument: type"),
    };
    
    let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("");
    let details = args.get("details").and_then(|v| v.as_str()).unwrap_or("");
    let project = args.get("project").and_then(|v| v.as_str()).unwrap_or("");
    
    // Cargar contrato si hay proyecto especificado
    let contract = if !project.is_empty() {
        match load_contract(project) {
            Ok(c) => Some(c),
            Err(_) => None,
        }
    } else {
        None
    };
    
    // Validar según tipo de acción
    let validation = match action_type {
        "edit" | "write" => validate_file_modification(target, details, contract.as_ref()),
        "commit" | "push" => validate_git_operation(details, contract.as_ref()),
        "deploy" => validate_deployment(contract.as_ref()),
        _ => ValidationResult {
            allow: true,
            reason: "Unknown action type — allowing by default".to_string(),
            constraints: vec![],
        },
    };
    
    let response = json!({
        "allow": validation.allow,
        "reason": validation.reason,
        "constraints": validation.constraints
    });
    
    ok_json(id, response)
}

pub fn propose_change_tool(id: Value, args: &Value) -> Value {
    let reason = match args.get("reason").and_then(|v| v.as_str()) {
        Some(r) => r,
        None => return tool_error(id, "Missing argument: reason"),
    };
    
    let diff = args.get("diff").and_then(|v| v.as_str()).unwrap_or("");
    let project = args.get("project").and_then(|v| v.as_str()).unwrap_or("");
    
    // Generar ID de propuesta
    let proposal_id = format!("prop-{}", generate_short_id());
    
    // Guardar propuesta en cola
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let inbox_dir = PathBuf::from(&home)
        .join("Documents/workspace-mcp-global/inbox");
    
    if let Err(e) = std::fs::create_dir_all(&inbox_dir) {
        return tool_error(id, &format!("Failed to create inbox dir: {}", e));
    }
    
    let proposal_path = inbox_dir.join(format!("{}.json", proposal_id));
    let proposal = json!({
        "id": proposal_id,
        "project": project,
        "reason": reason,
        "diff": diff,
        "timestamp": chrono_like_now(),
        "status": "pending"
    });
    
    if let Err(e) = fs::write(&proposal_path, serde_json::to_string_pretty(&proposal).unwrap()) {
        return tool_error(id, &format!("Failed to save proposal: {}", e));
    }
    
    let response = json!({
        "queued": true,
        "proposal_id": proposal_id,
        "message": format!("Propuesta encolada para aprobación humana. Revisa con /inbox")
    });
    
    ok_json(id, response)
}

struct ValidationResult {
    allow: bool,
    reason: String,
    constraints: Vec<String>,
}

fn validate_file_modification(
    _target: &str,
    details: &str,
    contract: Option<&Value>,
) -> ValidationResult {
    let mut constraints = Vec::new();
    
    // Verificar si el archivo está en out_of_scope
    if let Some(c) = contract {
        if let Some(out_of_scope) = c.get("out_of_scope").and_then(|o| o.as_array()) {
            for item in out_of_scope {
                if let Some(scope_item) = item.as_str() {
                    if details.to_lowercase().contains(&scope_item.to_lowercase()) {
                        return ValidationResult {
                            allow: false,
                            reason: format!("'{}' está fuera de alcance según el contrato", scope_item),
                            constraints: vec![],
                        };
                    }
                }
            }
        }
        
        // Cargar constraints aplicables
        if let Some(c_list) = c.get("constraints").and_then(|c| c.as_array()) {
            for constraint in c_list {
                if let Some(c_str) = constraint.as_str() {
                    constraints.push(c_str.to_string());
                }
            }
        }
    }
    
    // Por defecto, permitir modificaciones de archivos
    ValidationResult {
        allow: true,
        reason: "File modification allowed".to_string(),
        constraints,
    }
}

fn validate_git_operation(details: &str, contract: Option<&Value>) -> ValidationResult {
    // Verificar que no se esté commiteando algo fuera de scope
    if let Some(c) = contract {
        if let Some(out_of_scope) = c.get("out_of_scope").and_then(|o| o.as_array()) {
            for item in out_of_scope {
                if let Some(scope_item) = item.as_str() {
                    if details.to_lowercase().contains(&scope_item.to_lowercase()) {
                        return ValidationResult {
                            allow: false,
                            reason: format!("Commit incluye '{}' que está fuera de alcance", scope_item),
                            constraints: vec![],
                        };
                    }
                }
            }
        }
    }
    
    ValidationResult {
        allow: true,
        reason: "Git operation allowed".to_string(),
        constraints: vec![],
    }
}

fn validate_deployment(contract: Option<&Value>) -> ValidationResult {
    let mut constraints = Vec::new();
    
    if let Some(c) = contract {
        if let Some(criteria) = c.get("success_criteria") {
            if let Some(max_diff) = criteria.get("visual_diff_max").and_then(|v| v.as_f64()) {
                constraints.push(format!("Visual diff debe ser < {:.1}%", max_diff));
            }
            if let Some(max_audit) = criteria.get("npm_audit_critical").and_then(|v| v.as_u64()) {
                constraints.push(format!("npm audit critical debe ser <= {}", max_audit));
            }
        }
    }
    
    ValidationResult {
        allow: true,
        reason: "Deployment allowed — verify success criteria first".to_string(),
        constraints,
    }
}

fn load_contract(project: &str) -> Result<Value, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let contract_path = PathBuf::from(home)
        .join("Documents/workspace-mcp-global/contracts")
        .join(format!("{}.json", project));
    
    if !contract_path.is_file() {
        return Err(format!("No contract found for project '{}'", project));
    }
    
    let content = fs::read_to_string(&contract_path)
        .map_err(|e| format!("Failed to read contract: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid contract JSON: {}", e))
}

fn generate_short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:x}", timestamp)
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("ts={}", secs)
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

fn ok_json(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": false,
            "content": [
                { "type": "text", "text": serde_json::to_string_pretty(&result).unwrap() }
            ]
        }
    })
}
