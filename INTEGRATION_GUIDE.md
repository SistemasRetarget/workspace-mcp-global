# Guía de Integración: MCP Supervisor con Claude/Windsurf

**Fecha:** 2026-04-25  
**Objetivo:** Integrar las nuevas fat tools del MCP supervisor con Claude Code/Windsurf

---

## Estado Actual

### ✅ Completado
- [x] Plan de evolución documentado (`MCP_EVOLUTION_PLAN.md`)
- [x] Contrato inicial de puebloladehesa-rediseno creado
- [x] Módulos Rust implementados:
  - `tools/session.rs` → `session.start()`, `session.report()`
  - `tools/iterate.rs` → `iterate.section()` (loop autónomo)
  - `tools/contract.rs` → `validate-action()`, `propose-change()`

### ⏸️ Pendiente
- [ ] Compilar el MCP con los nuevos módulos
- [ ] Actualizar `main.rs` para exponer las nuevas tools
- [ ] Configurar hook pre-tool-use en Claude Code
- [ ] Crear slash commands (`/start`, `/approve`, `/reject`, `/inbox`)
- [ ] Setup de reportes automáticos a Drive + email

---

## Próximos Pasos para Compilar

### 1. Actualizar `main.rs`

Agregar las nuevas tools al dispatcher:

```rust
// En tool_definitions()
{
    "name": "session.start",
    "description": "Carga TODO el contexto del proyecto en una sola llamada: contrato, estado QA, lecciones, último commit, siguiente tarea.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "project": { "type": "string", "description": "Nombre del proyecto" }
        },
        "required": ["project"]
    }
},
{
    "name": "session.report",
    "description": "Genera reporte markdown estructurado del estado actual del proyecto.",
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
    "description": "FAT TOOL: Ejecuta loop autónomo de screenshot → diff → fix → commit → push hasta converger o agotar iteraciones.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "project": { "type": "string" },
            "section": { "type": "string", "description": "ID de la sección a iterar" },
            "deploy_url": { "type": "string", "description": "URL del deploy en Railway" },
            "max_iterations": { "type": "integer", "description": "Default 999" },
            "tolerance_percent": { "type": "number", "description": "Default 2.0" }
        },
        "required": ["project", "section", "deploy_url"]
    }
},
{
    "name": "contract.validate-action",
    "description": "Valida una acción contra el contrato antes de ejecutarla. Devuelve allow=false si está fuera de scope.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "project": { "type": "string" },
            "type": { "type": "string", "enum": ["edit", "write", "commit", "push", "deploy"] },
            "target": { "type": "string", "description": "Archivo o sección afectada" },
            "details": { "type": "string", "description": "Descripción de la acción" }
        },
        "required": ["type"]
    }
},
{
    "name": "contract.propose-change",
    "description": "Encola una propuesta de cambio fuera de scope para aprobación humana.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "project": { "type": "string" },
            "reason": { "type": "string", "description": "Por qué se necesita el cambio" },
            "diff": { "type": "string", "description": "Qué se agregaría/cambiaría" }
        },
        "required": ["reason"]
    }
}
```

### 2. Actualizar dispatcher en `handle_tool_call()`

```rust
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
    
    // NUEVAS FAT TOOLS
    "session.start" => tools::session::session_start_tool(id, &args, &workspace_root()),
    "session.report" => tools::session::session_report_tool(id, &args, &workspace_root()),
    "iterate.section" => tools::iterate::iterate_section_tool(id, &args),
    "contract.validate-action" => tools::contract::validate_action_tool(id, &args),
    "contract.propose-change" => tools::contract::propose_change_tool(id, &args),
    
    _ => tool_error(id, &format!("Unknown tool: {}", tool)),
}
```

### 3. Agregar imports al inicio de `main.rs`

```rust
mod tools;
use tools::session;
use tools::iterate;
use tools::contract;
```

### 4. Compilar

```bash
cd ~/Documents/workspace-mcp-global/servers/quality-gate
cargo build --release
```

### 5. Reiniciar MCP en Windsurf

Cerrar y reabrir Windsurf para que cargue el MCP actualizado.

---

## Uso desde Claude/Windsurf

### Iniciar sesión de trabajo

```
Claude: Llama a session.start con project="puebloladehesa-rediseno"
```

**Output esperado:**
```
Session started for project: puebloladehesa-rediseno
Contract version: 1
Last commit: f46e326
Sections DONE: 3
Sections PENDING: 5
Recent lessons: 1
Next task: iterate section: casas-grid
Antipatterns loaded: 0

Full context loaded. Ready to execute.
```

### Iterar una sección autónomamente

```
Claude: Llama a iterate.section con:
{
  "project": "puebloladehesa-rediseno",
  "section": "casas-grid",
  "deploy_url": "https://puebloladehesa-web-production.up.railway.app",
  "max_iterations": 999,
  "tolerance_percent": 2.0
}
```

**Output esperado (después de N iteraciones):**
```json
{
  "status": "CONVERGED",
  "iterations": 7,
  "final_diff_percent": 1.8,
  "commits": ["iter-casas-grid-1", "iter-casas-grid-2", ...],
  "message": "Section 'casas-grid' converged after 7 iterations (diff: 1.8%)"
}
```

### Validar acción antes de ejecutar

```
Claude: Antes de editar archivo, llama a contract.validate-action:
{
  "project": "puebloladehesa-rediseno",
  "type": "edit",
  "target": "src/app/cart/page.tsx",
  "details": "agregar página de carrito de compras"
}
```

**Output esperado:**
```json
{
  "allow": false,
  "reason": "'ecommerce / carrito' está fuera de alcance según el contrato",
  "constraints": []
}
```

### Proponer cambio fuera de scope

```
Claude: Llama a contract.propose-change:
{
  "project": "puebloladehesa-rediseno",
  "reason": "Usuario pidió agregar carrito de compras",
  "diff": "Agregar /cart route + checkout flow + integración con Stripe"
}
```

**Output esperado:**
```json
{
  "queued": true,
  "proposal_id": "prop-661a3f2c",
  "message": "Propuesta encolada para aprobación humana. Revisa con /inbox"
}
```

### Generar reporte

```
Claude: Llama a session.report con project="puebloladehesa-rediseno"
```

**Output esperado:**
```markdown
# Reporte de Progreso: puebloladehesa-rediseno

**Fecha:** 2026-04-25 23:56:00 UTC
**Último commit:** `f46e326`

## Estado de Secciones

| Sección | Estado | Notas |
|---------|--------|-------|
| ✅ header | DONE | Sticky transparent→cream, logo+nav visibles |
| ✅ hero | DONE | Cordillera visible, título centrado |
| ✅ intro-paragraph | DONE | Texto alineado con producción |
| ⏸️ casas-grid | PENDING | Prod usa /estadias, QA usa /casas |
| ⏸️ experiencias | PENDING |  |
| ⏸️ testimonios | PENDING |  |
| ⏸️ faq | PENDING |  |
| ⏸️ footer | PENDING |  |

## Métricas

- **Secciones completadas:** 3
- **Secciones pendientes:** 5
- **Iteraciones totales:** 8
- **Lecciones registradas:** 1

## Próximos Pasos

- iterate section: casas-grid
```

---

## Slash Commands (Pendiente de implementar)

### `/start <project>`
Wrapper de `session.start()`

### `/iterate <section>`
Wrapper de `iterate.section()` con valores por defecto del contrato

### `/approve <section>`
Marca sección como DONE en el contrato y avanza a la siguiente

### `/reject <section> "<feedback>"`
Re-encola la sección con feedback del usuario

### `/inbox`
Lista propuestas pendientes de aprobación

### `/report`
Wrapper de `session.report()`

---

## Reportes Automáticos (Pendiente)

### Setup de Drive público

1. Crear carpeta en Drive: `RETARGET-PUBLIC/`
2. Compartir con acceso público de lectura
3. Obtener URL compartible
4. Configurar en MCP para escribir reportes ahí

### Setup de emails automáticos

1. Configurar Gmail API o SMTP
2. Crear tareas programadas:
   - 12:00 PM → `session.report()` + email
   - 5:00 PM → `session.report()` + email
3. Template de email:
   ```
   Asunto: [Retarget] Reporte de Progreso - <proyecto> - <hora>
   
   Hola Luis,
   
   Aquí está el reporte de progreso de <proyecto>:
   
   <contenido del reporte markdown>
   
   Ver reporte completo: <link a Drive público>
   
   ---
   Generado automáticamente por MCP Supervisor
   ```

---

## Diferencias Clave vs. Thin Tools

| Aspecto | Thin Tools (actual) | Fat Tools (nuevo) |
|---------|---------------------|-------------------|
| Turnos por sección | 20-30 | 1-3 |
| Contexto cargado | Manual (múltiples reads) | Automático (session.start) |
| Iteración | Manual (Claude llama cada paso) | Autónoma (MCP loopea internamente) |
| Enforcement | Sin bloqueo | Pre-validación con contrato |
| Reportes | Manual | Automático 2x/día |
| Propuestas fuera de scope | Claude ejecuta igual | Bloqueadas + encoladas |

---

## Próxima Sesión de Trabajo

1. **Compilar MCP actualizado**
2. **Probar `session.start()` con puebloladehesa-rediseno**
3. **Probar `iterate.section()` en casas-grid** (simulado, sin Claude patch aún)
4. **Validar que `contract.validate-action()` bloquea acciones fuera de scope**
5. **Generar primer reporte con `session.report()`**

**Estimación:** 1-2 horas para compilar + probar ciclo básico
