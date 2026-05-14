//! Tickets fat tools — Retarget Gantt Sheet
//!
//! Writes/reads tickets directly to/from the Google Sheets Gantt via python3 subprocess.
//!   - ticket_create   Open a new ticket row in the Sheet → returns T-XX id
//!   - ticket_close    Update status + close date + vitácora column
//!   - ticket_search   Full-text search across all ticket rows

use serde_json::{json, Value};
use std::process::Command;

// ─── Constants ────────────────────────────────────────────────────────────────

const SHEETS_ID: &str = "1murmG-pdc5GkJ1CYc4_1UISRTcipMxPYv2jiH_-7ZIY";
const SHEET_TAB: &str = "Retarget · Gantt Tareas Web — Mayo 2026";
const SA_KEY_PATH: &str =
    "/Users/spam11/Desktop/RETARGET-WORKSPACE/retarget-mcp-2d37bb49c600.json";

// ─── Shared helper: run python3 snippet ───────────────────────────────────────

fn run_python(script: &str) -> Result<String, String> {
    let out = Command::new("python3")
        .args(["-c", script])
        .output()
        .map_err(|e| format!("python3 spawn failed: {e}"))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// ─── Tool: ticket_create ──────────────────────────────────────────────────────

/// Opens a new ticket row in the Gantt Sheet.
/// Args: site (str), origen (str, e.g. "Email Leig"), descripcion (str)
/// Returns: {"ticket_id": "T-XX", "row": N}
pub fn ticket_create_tool(id: Value, args: &Value) -> Value {
    let site = args.get("site").and_then(|v| v.as_str()).unwrap_or("—");
    let origen = args.get("origen").and_then(|v| v.as_str()).unwrap_or("Manual");
    let descripcion = match args.get("descripcion").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return tool_error(id, "Missing argument: descripcion"),
    };

    // Escape single quotes in strings to avoid breaking the python script
    let site_e = site.replace('\'', "\\'");
    let origen_e = origen.replace('\'', "\\'");
    let desc_e = descripcion.replace('\'', "\\'");

    let script = format!(
        r#"
import json, sys
from datetime import date
from google.oauth2.service_account import Credentials
from googleapiclient.discovery import build

creds = Credentials.from_service_account_file(
    '{sa}',
    scopes=['https://www.googleapis.com/auth/spreadsheets']
)
svc = build('sheets', 'v4', credentials=creds, cache_discovery=False)
sheet = svc.spreadsheets()

# Read current rows to determine next T-XX
result = sheet.values().get(
    spreadsheetId='{sid}',
    range='{tab}!A:A'
).execute()
rows = result.get('values', [])

max_num = 0
for r in rows:
    if r and r[0].startswith('T-'):
        try:
            n = int(r[0][2:])
            if n > max_num:
                max_num = n
        except ValueError:
            pass

ticket_id = f'T-{{max_num + 1}}'
today = date.today().isoformat()

# Append new row: Ticket | Estado | Sitio | Origen | Descripción | F.Apertura | F.Cierre | Verificado | Bitácora
new_row = [ticket_id, 'Pendiente', '{site}', '{origen}', '{desc}', today, '', '', '']

sheet.values().append(
    spreadsheetId='{sid}',
    range='{tab}!A:I',
    valueInputOption='USER_ENTERED',
    body={{'values': [new_row]}}
).execute()

# Find the row number we just wrote
rows_after = sheet.values().get(
    spreadsheetId='{sid}',
    range='{tab}!A:A'
).execute().get('values', [])

row_num = len(rows_after)  # Last row appended
print(json.dumps({{'ticket_id': ticket_id, 'row': row_num, 'today': today}}))
"#,
        sa = SA_KEY_PATH,
        sid = SHEETS_ID,
        tab = SHEET_TAB,
        site = site_e,
        origen = origen_e,
        desc = desc_e,
    );

    match run_python(&script) {
        Ok(output) => {
            let data: Value = serde_json::from_str(&output).unwrap_or(json!({"raw": output}));
            let ticket_id = data.get("ticket_id").and_then(|v| v.as_str()).unwrap_or("?");
            ok_json(
                id,
                &format!("Ticket {} created successfully.", ticket_id),
                data,
            )
        }
        Err(e) => tool_error(id, &format!("ticket_create failed: {e}")),
    }
}

// ─── Tool: ticket_close ───────────────────────────────────────────────────────

/// Closes a ticket: sets estado=Listo, fecha_cierre=today, writes vitácora.
/// Args: ticket_id (str, e.g. "T-14"), bitacora (str), estado (str, default "Listo")
pub fn ticket_close_tool(id: Value, args: &Value) -> Value {
    let ticket_id = match args.get("ticket_id").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return tool_error(id, "Missing argument: ticket_id"),
    };
    let bitacora = args
        .get("bitacora")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let estado = args
        .get("estado")
        .and_then(|v| v.as_str())
        .unwrap_or("Listo");

    let tid_e = ticket_id.replace('\'', "\\'");
    let vit_e = bitacora.replace('\'', "\\'");
    let est_e = estado.replace('\'', "\\'");

    let script = format!(
        r#"
import json, sys
from datetime import date
from google.oauth2.service_account import Credentials
from googleapiclient.discovery import build

creds = Credentials.from_service_account_file(
    '{sa}',
    scopes=['https://www.googleapis.com/auth/spreadsheets']
)
svc = build('sheets', 'v4', credentials=creds, cache_discovery=False)
sheet = svc.spreadsheets()

# Find ticket row
result = sheet.values().get(
    spreadsheetId='{sid}',
    range='{tab}!A:A'
).execute()
rows = result.get('values', [])

row_index = None
for i, r in enumerate(rows):
    if r and r[0] == '{tid}':
        row_index = i + 1  # 1-based
        break

if row_index is None:
    print(json.dumps({{'error': 'Ticket {tid} not found'}}))
    sys.exit(1)

today = date.today().isoformat()

# Col B (estado) = index 2, Col G (F.Cierre) = index 7, Col I (Bitácora) = index 9
# Update estado (col B)
sheet.values().update(
    spreadsheetId='{sid}',
    range=f'{tab}!B{{row_index}}',
    valueInputOption='USER_ENTERED',
    body={{'values': [['{est}']]}}
).execute()

# Update F.Cierre (col G)
sheet.values().update(
    spreadsheetId='{sid}',
    range=f'{tab}!G{{row_index}}',
    valueInputOption='USER_ENTERED',
    body={{'values': [[today]]}}
).execute()

# Update Bitácora (col I)
if '{vit}':
    sheet.values().update(
        spreadsheetId='{sid}',
        range=f'{tab}!I{{row_index}}',
        valueInputOption='USER_ENTERED',
        body={{'values': [['{vit}']]}}
    ).execute()

print(json.dumps({{'ticket_id': '{tid}', 'row': row_index, 'estado': '{est}', 'fecha_cierre': today}}))
"#,
        sa = SA_KEY_PATH,
        sid = SHEETS_ID,
        tab = SHEET_TAB,
        tid = tid_e,
        est = est_e,
        vit = vit_e,
    );

    match run_python(&script) {
        Ok(output) => {
            let data: Value = serde_json::from_str(&output).unwrap_or(json!({"raw": output}));
            if data.get("error").is_some() {
                return tool_error(id, &format!("{}", data["error"]));
            }
            ok_json(id, &format!("Ticket {} closed → {}", ticket_id, estado), data)
        }
        Err(e) => tool_error(id, &format!("ticket_close failed: {e}")),
    }
}

// ─── Tool: ticket_search ──────────────────────────────────────────────────────

/// Full-text search across all Gantt ticket rows.
/// Args: query (str) — searches across all columns case-insensitively
/// Returns: matching rows as array
pub fn ticket_search_tool(id: Value, args: &Value) -> Value {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return tool_error(id, "Missing argument: query"),
    };

    let query_e = query.replace('\'', "\\'").to_lowercase();

    let script = format!(
        r#"
import json
from google.oauth2.service_account import Credentials
from googleapiclient.discovery import build

creds = Credentials.from_service_account_file(
    '{sa}',
    scopes=['https://www.googleapis.com/auth/spreadsheets.readonly']
)
svc = build('sheets', 'v4', credentials=creds, cache_discovery=False)
sheet = svc.spreadsheets()

result = sheet.values().get(
    spreadsheetId='{sid}',
    range='{tab}!A:I'
).execute()
rows = result.get('values', [])

query = '{q}'
headers = rows[0] if rows else []
matches = []

for i, row in enumerate(rows[1:], start=2):
    row_str = ' '.join(row).lower()
    if query in row_str:
        row_dict = {{}}
        for j, header in enumerate(headers):
            row_dict[header] = row[j] if j < len(row) else ''
        row_dict['_row'] = i
        matches.append(row_dict)

print(json.dumps({{'query': '{q}', 'count': len(matches), 'results': matches}}))
"#,
        sa = SA_KEY_PATH,
        sid = SHEETS_ID,
        tab = SHEET_TAB,
        q = query_e,
    );

    match run_python(&script) {
        Ok(output) => {
            let data: Value = serde_json::from_str(&output).unwrap_or(json!({"raw": output}));
            let count = data.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
            ok_json(id, &format!("Found {} ticket(s) matching '{}'", count, query), data)
        }
        Err(e) => tool_error(id, &format!("ticket_search failed: {e}")),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

fn ok_json(id: Value, summary: &str, data: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": false,
            "content": [
                { "type": "text", "text": summary },
                { "type": "resource", "resource": {
                    "uri": "tickets://gantt",
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                }}
            ]
        }
    })
}
