//! Autonomous iteration tool
//!
//! FAT TOOL que ejecuta el loop completo: screenshot → diff → fix → commit → push → wait → repeat

use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const STAGNATION_LIMIT: usize = 3;
const RAILWAY_BUILD_WAIT_SECONDS: u64 = 180;

pub fn iterate_section_tool(id: Value, args: &Value) -> Value {
    let section = match args.get("section").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_error(id, "Missing argument: section"),
    };
    
    let max_iterations = args
        .get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(999) as usize;
    
    let tolerance = args
        .get("tolerance_percent")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);
    
    let project = match args.get("project").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return tool_error(id, "Missing argument: project"),
    };
    
    let deploy_url = match args.get("deploy_url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return tool_error(id, "Missing argument: deploy_url"),
    };
    
    // Ejecutar loop autónomo
    match run_iteration_loop(project, section, deploy_url, max_iterations, tolerance) {
        Ok(result) => ok_json(id, result),
        Err(e) => tool_error(id, &e),
    }
}

#[allow(dead_code)]
struct IterationResult {
    iteration: usize,
    diff_percent: f64,
    improved: bool,
    commit: String,
}

fn run_iteration_loop(
    project: &str,
    section: &str,
    deploy_url: &str,
    max_iterations: usize,
    tolerance: f64,
) -> Result<Value, String> {
    let mut iterations = Vec::new();
    let mut stagnation_count = 0;
    let mut last_diff = 100.0;
    
    for i in 1..=max_iterations {
        eprintln!("[iterate] Iteration {} for section '{}'", i, section);
        
        // 1. Esperar build si no es la primera iteración
        if i > 1 {
            eprintln!("[iterate] Waiting {} seconds for Railway build...", RAILWAY_BUILD_WAIT_SECONDS);
            thread::sleep(Duration::from_secs(RAILWAY_BUILD_WAIT_SECONDS));
        }
        
        // 2. Capturar screenshot
        let screenshot_result = capture_screenshot(project, section, deploy_url)?;
        eprintln!("[iterate] Screenshot captured: {}", screenshot_result);
        
        // 3. Comparar con referencia
        let diff_result = compare_with_reference(project, section, tolerance)?;
        let current_diff = diff_result.diff_percent;
        let passed = diff_result.passed;
        
        eprintln!("[iterate] Visual diff: {:.2}% (tolerance: {:.2}%)", current_diff, tolerance);
        
        // 4. Verificar convergencia
        if passed {
            iterations.push(IterationResult {
                iteration: i,
                diff_percent: current_diff,
                improved: true,
                commit: "converged".to_string(),
            });
            
            return Ok(json!({
                "status": "CONVERGED",
                "iterations": i,
                "final_diff_percent": current_diff,
                "commits": extract_commits(&iterations),
                "message": format!("Section '{}' converged after {} iterations (diff: {:.2}%)", section, i, current_diff)
            }));
        }
        
        // 5. Verificar estancamiento
        let improved = current_diff < last_diff - 0.5; // Mejora mínima de 0.5%
        if !improved {
            stagnation_count += 1;
            eprintln!("[iterate] No improvement detected ({}/{})", stagnation_count, STAGNATION_LIMIT);
            
            if stagnation_count >= STAGNATION_LIMIT {
                return Ok(json!({
                    "status": "STAGNATION",
                    "iterations": i,
                    "final_diff_percent": current_diff,
                    "commits": extract_commits(&iterations),
                    "message": format!("Section '{}' stagnated after {} iterations (diff stuck at {:.2}%)", section, i, current_diff)
                }));
            }
        } else {
            stagnation_count = 0; // Reset si hay mejora
        }
        
        // 6. Buscar lecciones aplicables
        let lessons = search_lessons(section)?;
        
        // 7. AQUÍ LLAMARÍAMOS A CLAUDE PARA GENERAR EL PATCH
        // Por ahora, simulamos que el patch ya fue aplicado manualmente
        eprintln!("[iterate] Would request patch from Claude here...");
        eprintln!("[iterate] Lessons found: {}", lessons.len());
        
        // 8. Commit + push (simulado por ahora)
        let commit_hash = format!("iter-{}-{}", section, i);
        
        iterations.push(IterationResult {
            iteration: i,
            diff_percent: current_diff,
            improved,
            commit: commit_hash.clone(),
        });
        
        last_diff = current_diff;
    }
    
    // Alcanzó max_iterations sin converger
    Ok(json!({
        "status": "MAX_ITERATIONS",
        "iterations": max_iterations,
        "final_diff_percent": last_diff,
        "commits": extract_commits(&iterations),
        "message": format!("Section '{}' reached max iterations ({}) without converging (final diff: {:.2}%)", section, max_iterations, last_diff)
    }))
}

fn capture_screenshot(project: &str, view: &str, url: &str) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let evidence_dir = PathBuf::from(&home)
        .join("Documents/workspace-mcp-global/evidence")
        .join(project)
        .join(view);
    
    std::fs::create_dir_all(&evidence_dir)
        .map_err(|e| format!("Failed to create evidence dir: {}", e))?;
    
    let output_path = evidence_dir.join("actual.png");
    let screenshot_url = format!("{}?screenshot=1", url);
    
    let output = Command::new("npx")
        .args(&[
            "playwright",
            "screenshot",
            "--viewport-size=1280,1080",
            "--wait-for-timeout=1500",
            &screenshot_url,
            output_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Failed to run playwright: {}", e))?;
    
    if !output.status.success() {
        return Err(format!(
            "Screenshot failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(output_path.display().to_string())
}

struct DiffResult {
    passed: bool,
    diff_percent: f64,
}

fn compare_with_reference(project: &str, view: &str, tolerance: f64) -> Result<DiffResult, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let evidence_dir = PathBuf::from(&home)
        .join("Documents/workspace-mcp-global/evidence")
        .join(project)
        .join(view);
    
    let reference_path = evidence_dir.join("reference.png");
    let actual_path = evidence_dir.join("actual.png");
    let diff_path = evidence_dir.join("diff.png");
    
    if !reference_path.is_file() {
        return Err(format!("No reference found at {}", reference_path.display()));
    }
    
    if !actual_path.is_file() {
        return Err(format!("No actual screenshot at {}", actual_path.display()));
    }
    
    let script_path = PathBuf::from(&home)
        .join("Documents/workspace-mcp-global/tools/visual-diff.mjs");
    
    if !script_path.is_file() {
        return Err(format!("visual-diff.mjs not found at {}", script_path.display()));
    }
    
    let output = Command::new("node")
        .args(&[
            script_path.to_str().unwrap(),
            reference_path.to_str().unwrap(),
            actual_path.to_str().unwrap(),
            diff_path.to_str().unwrap(),
            &tolerance.to_string(),
        ])
        .output()
        .map_err(|e| format!("Failed to run visual-diff: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse diff result: {}", e))?;
    
    let passed = result.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
    let diff_percent = result.get("diff_percent").and_then(|v| v.as_f64()).unwrap_or(100.0);
    
    Ok(DiffResult { passed, diff_percent })
}

fn search_lessons(section: &str) -> Result<Vec<Value>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let lessons_path = PathBuf::from(&home)
        .join("Documents/workspace-mcp-global/lessons/lessons.jsonl");
    
    if !lessons_path.is_file() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(&lessons_path)
        .map_err(|e| format!("Failed to read lessons: {}", e))?;
    
    let query = section.to_lowercase();
    let lessons: Vec<Value> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|v: &Value| {
            let blob = format!(
                "{} {} {}",
                v.get("category").and_then(|c| c.as_str()).unwrap_or(""),
                v.get("symptom").and_then(|s| s.as_str()).unwrap_or(""),
                v.get("fix").and_then(|f| f.as_str()).unwrap_or(""),
            )
            .to_lowercase();
            blob.contains(&query)
        })
        .take(5)
        .collect();
    
    Ok(lessons)
}

fn extract_commits(iterations: &[IterationResult]) -> Vec<String> {
    iterations.iter().map(|i| i.commit.clone()).collect()
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
