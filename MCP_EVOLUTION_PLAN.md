# Plan de Evolución: quality-gate → supervisor

**Fecha:** 2026-04-25  
**Objetivo:** Transformar el MCP de validador reactivo a supervisor autónomo que controla el flujo completo de desarrollo

---

## Estado Actual del MCP

### Tools existentes (thin tools)
```
✅ list-projects       → descubre proyectos
✅ validate            → typecheck + lint + build (blocking)
✅ run-tests           → ejecuta suite de tests
✅ logs                → lee últimas N líneas de logs
✅ railway-logs        → tail de logs de Railway
✅ reference-set       → guarda imagen de referencia
✅ screenshot          → captura URL con Playwright
✅ visual-diff         → compara actual vs referencia
✅ lessons-log         → registra lección en KB
✅ lessons-search      → busca lecciones previas
```

### Problemas identificados

1. **Consumo de turnos:** Cada tool es atómica → Claude necesita 5-10 turnos para un ciclo completo
2. **Sin contexto persistente:** MCP no guarda estado del proyecto activo
3. **Sin enforcement del contrato:** Claude puede salirse del scope sin bloqueo
4. **Sin orquestación:** No hay tools "gordas" que ejecuten workflows completos
5. **Sin auto-iteración:** Claude debe llamar manualmente screenshot → diff → fix → commit → repeat

---

## Arquitectura Objetivo: Supervisor

### Principios de diseño

1. **Fat tools, not thin:** Una tool = un workflow completo
2. **Contexto en memoria:** MCP carga contrato + lecciones + estado al inicio
3. **Contract-driven execution:** Toda acción pasa por `validate-action` antes de ejecutarse
4. **Auto-iteración interna:** MCP loopea internamente, Claude solo genera patches cuando se necesitan
5. **Reporte estructurado:** Output final es markdown profesional para Drive público

---

## Nuevas Tools (Fat Tools)

### 1. Session Management

#### `session.start(project)`
**Input:**
```json
{
  "project": "puebloladehesa-rediseno"
}
```

**Output:**
```json
{
  "contract": { /* contract.json completo */ },
  "state": { /* .qa-state.json */ },
  "last_commit": "f46e326",
  "lessons": [ /* 5 lecciones más relevantes */ ],
  "next_task": "iterate casas-grid section",
  "antipatterns": [ /* antipatrones aplicables */ ]
}
```

**Función:** Carga TODO el contexto del proyecto en una sola llamada. Claude recibe el contrato, estado, lecciones y siguiente tarea sin tener que leer múltiples archivos.

---

#### `session.report()`
**Output:** Markdown estructurado con:
- Commits realizados
- Secciones completadas/pendientes
- Lighthouse scores
- Visual diff results
- Lecciones registradas
- Próximos pasos

**Función:** Genera el reporte diario para Drive público + email automático.

---

### 2. Contract Enforcement

#### `contract.validate-action(action)`
**Input:**
```json
{
  "type": "edit" | "write" | "commit" | "push" | "deploy",
  "target": "/path/to/file" | "section-name",
  "details": "descripción de la acción"
}
```

**Output:**
```json
{
  "allow": true | false,
  "reason": "explicación si allow=false",
  "constraints": [ /* restricciones aplicables */ ]
}
```

**Función:** Hook pre-tool-use. Bloquea acciones fuera del contrato ANTES de ejecutarse.

---

#### `contract.propose-change(reason, diff)`
**Input:**
```json
{
  "reason": "user pidió agregar carrito de compras",
  "diff": "agregar /cart route + checkout flow"
}
```

**Output:**
```json
{
  "queued": true,
  "proposal_id": "prop-001",
  "message": "Propuesta encolada para aprobación humana"
}
```

**Función:** Cuando Claude detecta que necesita salir del scope, NO ejecuta → encola para aprobación.

---

### 3. Autonomous Iteration

#### `iterate.section(section_id, max_iterations)`
**Input:**
```json
{
  "section": "casas-grid",
  "max_iterations": 999,
  "tolerance_percent": 2.0
}
```

**Workflow interno:**
```
1. Captura screenshot de deploy
2. Compara con referencia
3. Si diff > tolerance:
   a. Busca lecciones aplicables
   b. Pide a Claude un patch específico
   c. Aplica patch
   d. Commit + push
   e. Espera Railway build (3 min)
   f. Vuelve a 1
4. Si 3 iteraciones consecutivas no mejoran → STAGNATION
5. Si diff <= tolerance → CONVERGED
```

**Output:**
```json
{
  "status": "CONVERGED" | "STAGNATION" | "MAX_ITERATIONS",
  "iterations": 7,
  "final_diff_percent": 1.2,
  "commits": ["abc123", "def456", ...],
  "duration_minutes": 24
}
```

**Función:** **ESTO ES LA CLAVE.** Un solo turno de Claude dispara el loop completo. MCP itera autónomamente hasta converger o agotar límite.

---

### 4. Clone-Site Workflow

#### `clone.bootstrap(url)`
**Input:**
```json
{
  "url": "https://puebloladehesa.cl"
}
```

**Workflow interno:**
```
1. Fetch sitemap.xml
2. Descubre landings + contenido
3. Extrae URLs de CDN (imágenes, videos, fonts)
4. Detecta plataforma (WordPress, Shopify, Next.js, etc.)
5. Descarga assets a public/media/
6. Captura screenshots de todas las vistas
7. Genera brand-palette.json (colores, tipografía)
8. Crea chunk-plan.json (división en secciones manejables)
9. Genera prompts/ para cada chunk
```

**Output:**
```json
{
  "site_type": "shopify",
  "landings": ["home", "estadias", "experiencias", "contacto"],
  "assets_downloaded": 47,
  "missing_content": [ /* lista de contenido faltante */ ],
  "chunks": [
    { "id": "header", "prompt": "prompts/header.md" },
    { "id": "hero", "prompt": "prompts/hero.md" },
    ...
  ]
}
```

**Función:** Fase 1-4 de la metodología clone-site en una sola tool call.

---

#### `clone.next-chunk()`
**Workflow interno:**
```
1. Lee chunk-plan.json
2. Toma el siguiente chunk PENDING
3. Ejecuta iterate.section(chunk_id) internamente
4. Marca chunk como LAYOUT_OK
5. Carga contenido aprobado → CONTENT_OK
6. Aplica interacciones + colores → DONE
```

**Output:**
```json
{
  "chunk": "casas-grid",
  "status": "DONE",
  "iterations": 5,
  "diff_percent": 1.8,
  "next_chunk": "testimonios"
}
```

**Función:** Fase 5-7 de clone-site. Claude solo dice `/clone-next` y el MCP ejecuta todo el ciclo.

---

### 5. Audit & Security

#### `audit.full(project)`
**Workflow interno:**
```
1. npm audit (high/critical)
2. secrets-scan (busca API keys, tokens)
3. tsc --noEmit
4. npm run lint
5. lighthouse (LCP, TBT, CLS)
6. visual-diff de todas las secciones vs prod
```

**Output:**
```json
{
  "passed": false,
  "npm_audit": { "high": 0, "critical": 1 },
  "secrets_found": ["ghp_xxx en .git/config"],
  "typecheck": "PASS",
  "lint": "3 warnings",
  "lighthouse": { "lcp": 2.1, "tbt": 180 },
  "visual_diffs": { "hero": 1.2, "casas-grid": 3.4 }
}
```

**Función:** Gate completo antes de deploy a producción. Un solo turno.

---

## Migración por Fases

### Fase 1: Fundación (1 semana)
- [ ] Crear `contracts/<project>.json` schema
- [ ] Implementar `session.start()` y `session.report()`
- [ ] Agregar `contract.validate-action()` stub
- [ ] Configurar Drive público + estructura de reportes
- [ ] Setup email automático (12pm y 5pm)

### Fase 2: Enforcement (1 semana)
- [ ] Implementar `contract.validate-action()` completo
- [ ] Implementar `contract.propose-change()`
- [ ] Hook pre-tool-use en Claude Code
- [ ] Crear `/approve` y `/reject` slash commands
- [ ] Crear `/inbox` para propuestas encoladas

### Fase 3: Auto-iteración (1 semana)
- [ ] Implementar `iterate.section()` con loop interno
- [ ] Migrar `visual-diff` a ser llamada internamente por iterate
- [ ] Implementar STAGNATION detection (3 iteraciones sin mejora)
- [ ] Agregar `lessons-auto-trigger` cuando sección cierra con >1 iteración

### Fase 4: Clone-Site (1 semana)
- [ ] Implementar `clone.bootstrap()`
- [ ] Implementar `clone.next-chunk()`
- [ ] Crear `chunk-plan.json` schema
- [ ] Integrar con `iterate.section()` para construcción de chunks
- [ ] Agregar `/clone-bootstrap` y `/clone-next` slash commands

### Fase 5: Audit & Polish (1 semana)
- [ ] Implementar `audit.full()`
- [ ] Agregar secrets-scan con regex patterns
- [ ] Integrar Lighthouse via Playwright
- [ ] Crear gate pre-deploy automático
- [ ] Documentar metodología completa

---

## Estructura de Archivos

```
workspace-mcp-global/
├── servers/
│   └── quality-gate/
│       ├── src/
│       │   ├── main.rs                    # Dispatcher MCP
│       │   ├── tools/
│       │   │   ├── session.rs             # session.start, session.report
│       │   │   ├── contract.rs            # validate-action, propose-change
│       │   │   ├── iterate.rs             # iterate.section (FAT TOOL)
│       │   │   ├── clone.rs               # clone.bootstrap, clone.next-chunk
│       │   │   ├── audit.rs               # audit.full
│       │   │   └── legacy.rs              # thin tools actuales
│       │   └── models/
│       │       ├── contract.rs            # Contract schema
│       │       ├── state.rs               # QA state schema
│       │       └── chunk_plan.rs          # Chunk plan schema
│       └── Cargo.toml
├── contracts/
│   ├── puebloladehesa-rediseno.json       # Contrato del proyecto
│   └── news-ai-cms.json
├── evidence/                              # Ya existe
├── lessons/                               # Ya existe
├── tools/                                 # Scripts auxiliares
│   ├── visual-diff.mjs                    # Ya existe
│   ├── secrets-scan.sh                    # Nuevo
│   └── lighthouse-batch.mjs               # Nuevo
└── reports/                               # Nuevo: reportes diarios
    └── puebloladehesa-rediseno/
        ├── 2026-04-25-mediodía.md
        └── 2026-04-25-cierre.md
```

---

## Contrato Ejemplo: puebloladehesa-rediseno.json

```json
{
  "project": "puebloladehesa-rediseno",
  "version": 1,
  "created": "2026-04-25",
  "owner": "sistemas@retarget.cl",
  
  "stack": {
    "frontend": "Next.js 15",
    "cms": "Payload CMS 3",
    "hosting": "Railway",
    "db": "Postgres",
    "locked": true
  },
  
  "design": {
    "reference_url": "https://puebloladehesa.cl",
    "tolerance_percent": 2.0,
    "responsive": ["mobile", "tablet", "desktop"],
    "brand": {
      "bg": "#F5EFE0",
      "ink": "#2A2A2A",
      "accent": "#D97757"
    }
  },
  
  "sections": [
    { "id": "header", "status": "DONE", "must_be_sticky": true },
    { "id": "hero", "status": "DONE", "image": "amplios_horizontes_1.webp" },
    { "id": "casas-grid", "status": "PENDING", "max_iterations": 999 },
    { "id": "experiencias", "status": "PENDING" },
    { "id": "footer", "status": "PENDING" }
  ],
  
  "constraints": [
    "header sticky: transparente arriba → crema al scroll (NO negro)",
    "todas las imágenes via next/image",
    "un solo contenedor GTM",
    "no scripts duplicados"
  ],
  
  "out_of_scope": [
    "ecommerce / carrito",
    "multi-idioma (solo ES)",
    "auth de usuarios finales",
    "blog (va en news-ai-cms)"
  ],
  
  "success_criteria": {
    "all_sections_status": "DONE",
    "visual_diff_max": 2.0,
    "npm_audit_critical": 0,
    "lighthouse_lcp_mobile": 2.5
  }
}
```

---

## Flujo de Trabajo Nuevo

### Día típico del ingeniero solitario

**Mañana (8am):**
- Email automático: agenda del día + estado de proyectos

**Inicio de sesión:**
```
Claude: /start puebloladehesa-rediseno
MCP: session.start() → carga contrato + estado + lecciones
Claude: [trabaja autónomo hasta agotar tokens]
```

**Mediodía (12pm):**
- Email automático: reporte de progreso
- Link a Drive público: `reports/puebloladehesa-rediseno/2026-04-25-mediodía.md`

**Tarde:**
```
Luis: /approve casas-grid
Claude: /clone-next
MCP: iterate.section("experiencias") → loop autónomo
```

**Cierre (5pm):**
- Email automático: reporte final del día
- Link a Drive: `reports/puebloladehesa-rediseno/2026-04-25-cierre.md`

**Interacciones totales:** 3-5 comandos/día vs. 50+ turnos actuales

---

## Métricas de Éxito

| Métrica | Actual | Objetivo |
|---------|--------|----------|
| Turnos por sección | 20-30 | 1-3 |
| Tiempo humano/día | 2-3 horas | 15-30 min |
| Iteraciones manuales | Todas | 0 (autónomas) |
| Reportes generados | Manual | Automático 2x/día |
| Acciones fuera de scope | Sin bloqueo | Bloqueadas pre-ejecución |
| Lecciones registradas | Manual | Auto-trigger |

---

## Próximos Pasos Inmediatos

1. **Crear contrato de puebloladehesa-rediseno** basado en PROJECT_STATUS.md actual
2. **Implementar `session.start()`** para cargar contexto en un turno
3. **Implementar `iterate.section()`** con loop interno básico
4. **Probar ciclo completo** en casas-grid section
5. **Generar primer reporte automático** para Drive

**Estimación:** 5 sprints de 1 semana = 1 mes para arquitectura completa

**Entregable final:** Plugin `retarget-solo-engineer` empaquetado + metodología documentada
