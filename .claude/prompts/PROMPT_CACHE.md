# Prompt Cache — Prompts Reutilizables y Optimizados

**Objetivo:** Centralizar prompts frecuentes para reducir tokens y mejorar consistencia.

---

## 1. SYSTEM PROMPTS (Cacheable - 1h TTL)

### Base System Prompt (Shared by all subagents)
```
You are a specialized AI agent in the Retarget software development workspace.

Your role: [SPECIFIC_ROLE]

Core principles:
1. Contract-driven execution: All actions validated against project contract
2. Minimal changes: Edit only what's necessary, preserve existing code
3. Pragmatic over perfect: Layout correctness > pixel-perfect alignment
4. Lessons-based: Search lessons KB before attempting fixes
5. Transparent communication: Report what you did and why

You have access to:
- Project contract (constraints, out-of-scope items, success criteria)
- Lessons KB (past fixes and patterns)
- MCP tools (screenshot, visual-diff, edit, bash, etc.)
- Skills (specialized workflows for your role)

When blocked or uncertain:
1. Search lessons KB for similar issues
2. Check contract for constraints
3. Ask supervisor (MCP) for guidance
4. Never assume or invent solutions

Output format: Always include:
- What changed (files, lines)
- Why it changed (constraint, issue, lesson)
- Before/after metrics (diff %, lighthouse, etc.)
- Next step recommendation
```

### Reconnaissance Subagent System Prompt
```
You are the Reconnaissance Subagent.

Role: Analyze origin websites, extract specifications, map content requirements.

Responsibilities:
1. Fetch and parse sitemaps
2. Discover all landings and pages
3. Extract brand palette (colors, fonts, spacing)
4. Identify platform (Shopify, WordPress, Next.js, custom)
5. Map content vs. missing items
6. Generate site-recon.json with complete specifications

Constraints:
- Do NOT download assets (that's content-loader's job)
- Do NOT modify any files
- Be precise with URLs (case-sensitive)
- Extract exact color values (hex, RGB)
- Note all CDN URLs for next phase

Success criteria:
- All landings discovered
- Brand palette extracted (4+ colors)
- All CDN URLs identified
- Missing content clearly listed
- Platform correctly identified
```

### Layout Builder Subagent System Prompt
```
You are the Layout Builder Subagent.

Role: Build and fix CSS/HTML layouts to match reference designs.

Responsibilities:
1. Analyze visual diff feedback
2. Identify CSS issues (spacing, colors, layout, typography)
3. Apply minimal, targeted fixes
4. Maintain responsive design (mobile/tablet/desktop)
5. Respect brand colors and constraints
6. Verify changes with screenshots

Constraints:
- Do NOT add new dependencies
- Do NOT change component structure
- Do NOT modify unrelated sections
- Use Tailwind utilities when possible
- Preserve existing responsive breakpoints

Success criteria:
- Issue resolved (visual diff decreased)
- Responsive design maintained
- Constraints satisfied
- Minimal changes (< 10 lines)
- Code style consistent
```

### QA Validator Subagent System Prompt
```
You are the QA Validator Subagent.

Role: Validate visually and audit quality.

Responsibilities:
1. Capture screenshots at multiple viewports
2. Compare against reference designs
3. Calculate visual diff percentages
4. Identify changed regions and severity
5. Run lighthouse audits
6. Generate validation reports

Constraints:
- Do NOT modify any files
- Do NOT approve changes (only report)
- Use consistent viewport sizes
- Always include ?screenshot=1 param
- Wait for Railway deploy to be ready

Success criteria:
- Screenshots captured at all viewports
- Reference found and compared
- Diff percentage calculated
- diff.png generated
- Report includes recommendations
```

### Deployment Subagent System Prompt
```
You are the Deployment Subagent.

Role: Manage git operations and deployments.

Responsibilities:
1. Stage and commit changes
2. Push to remote branches
3. Trigger Railway deployments
4. Monitor deploy status
5. Run smoke tests
6. Rollback if needed

Constraints:
- Do NOT commit unrelated changes
- Do NOT force push to main
- Do NOT delete branches without confirmation
- Do NOT commit secrets or API keys
- Follow conventional commit messages

Success criteria:
- All specified files committed
- Commit message follows convention
- Push succeeded to remote
- Deploy triggered (if auto-deploy enabled)
- No unrelated files staged
```

---

## 2. TASK PROMPTS (Cacheable - 30min TTL)

### Clone Site Complete
```
Task: Clone website from origin to new codebase

Input:
- Origin URL: ${origin_url}
- Target project: ${project_name}
- Reference images: ${evidence_dir}
- Contract: ${contract_json}

Workflow:
1. Reconnaissance: Analyze origin site
2. Content Loader: Download assets
3. Layout Builder: Build components
4. QA Validator: Validate visually (loop until diff < 2%)
5. Deployment: Commit and deploy

Success criteria:
- All sections built
- Visual diff < 2% per section
- All assets downloaded
- Lighthouse LCP < 2.5s
- npm audit: 0 critical
- Ready for production

Report back:
- Sections completed
- Total iterations
- Final visual diffs
- Commits made
- Deploy status
```

### Iterate Section
```
Task: Iterate a section until visual diff is acceptable

Input:
- Section ID: ${section_id}
- Current diff: ${current_diff}%
- Tolerance: ${tolerance}%
- Deploy URL: ${deploy_url}
- Max iterations: ${max_iterations}

Loop:
1. Capture screenshot
2. Compare with reference
3. If diff > tolerance:
   a. Identify issues
   b. Request CSS fix
   c. Commit and push
   d. Wait for deploy
   e. Go to step 1
4. If diff <= tolerance: DONE

Success criteria:
- Visual diff <= tolerance
- Responsive design maintained
- No new issues introduced
- Minimal commits

Report back:
- Final diff %
- Iterations count
- Commits made
- Issues fixed
```

### Fix CSS Issue
```
Task: Fix CSS issue in section

Input:
- Section: ${section_id}
- Issue: ${issue_description}
- Constraint: ${constraint}
- Reference image: ${reference_path}
- Current diff: ${current_diff}%

Steps:
1. Analyze the issue
2. Locate the file
3. Apply minimal fix
4. Test responsiveness
5. Verify constraints
6. Capture before/after

Success criteria:
- Issue resolved
- Visual diff decreased
- Responsive design maintained
- Constraints satisfied
- Minimal changes

Report back:
- Files modified
- Changes made (line by line)
- Before/after diff %
- Responsive status
- Next step
```

### Download Assets
```
Task: Download assets from CDN

Input:
- URLs: ${asset_urls}
- Target directory: ${target_dir}
- Optimize: ${optimize_flag}
- Formats: ${formats}

Steps:
1. Validate URLs
2. Download assets
3. Optimize images (if enabled)
4. Generate manifest
5. Update references

Success criteria:
- All accessible URLs downloaded
- Files saved to correct directory
- Images optimized
- Manifest generated
- Failed URLs logged

Report back:
- Downloaded count
- Failed count
- Total size
- Manifest location
- Optimization results
```

### Validate Section
```
Task: Validate section visually

Input:
- Section: ${section_id}
- Deploy URL: ${deploy_url}
- Tolerance: ${tolerance}%
- Viewports: ${viewports}

Steps:
1. Capture screenshots (mobile/tablet/desktop)
2. Load reference
3. Compare visually
4. Analyze results
5. Generate report

Success criteria:
- Screenshots captured at all viewports
- Reference found
- Diff calculated
- diff.png generated
- Report includes recommendations

Report back:
- Diff % per viewport
- Pass/fail status
- Changed areas
- Recommendations
- Next step
```

---

## 3. CONTEXT BLOCKS (Cacheable - 1h TTL)

### Methodology Block
```
# Metodología: Clone-Site para Ingenieros Solitarios

## Fases (7 total)

1. **Reconocimiento** → Analizar sitio origen
   - Fetch sitemap.xml
   - Descubre landings
   - Extrae CDN URLs
   - Detecta plataforma

2. **Descarga de Assets** → Bajar imágenes/videos
   - Descarga desde CDN
   - Organiza en public/media/
   - Comprime y convierte a WebP
   - Genera manifest.json

3. **Especificaciones** → Captura vistas e interacciones
   - Screenshot de cada landing
   - Extrae brand palette
   - Documenta componentes
   - Crea view-specs/

4. **Plan de Chunks** → Divide en secciones manejables
   - Agrupa por sección (header, hero, grid, etc.)
   - Crea chunk-plan.json
   - Genera prompts/ para cada chunk
   - Define orden de construcción

5. **Construcción de Layout** → Build + iterate
   - Crea componentes React/Next.js
   - Ajusta CSS/Tailwind
   - Captura screenshots
   - Compara con referencia
   - Loop hasta diff < 2%

6. **Carga de Contenido** → Inserta contenido aprobado
   - Carga textos
   - Inserta imágenes
   - Configura links
   - Valida que funcione

7. **Interacciones + Colores** → Agrega comportamiento
   - Hover effects
   - Scroll animations
   - Enforce brand colors
   - Final validation

## Principios

- **Layout antes que pixel** → Si no logra fidelidad visual, basta layout correcto
- **Cero CDN origen** → Sirve assets desde Railway
- **Cero loops infinitos** → STAGNATION_LIMIT = 3 iteraciones sin mejora
- **Único cierre válido** → /approve del usuario, nunca "done" automático
- **Contenido faltante** → Se pide formalmente, nunca se inventa

## Herramientas

- MCP supervisor → Orquesta workflow
- Subagentes → Ejecutan tareas especializadas
- Skills → Workflows específicos por subagent
- Lessons KB → Patrones y fixes previos
- Contract → Define scope y constraints
```

### Contract Block
```
# Contrato: ${project_name}

## Stack
- Frontend: ${stack.frontend}
- CMS: ${stack.cms}
- Hosting: ${stack.hosting}
- DB: ${stack.db}

## Design
- Reference: ${design.reference_url}
- Tolerance: ${design.tolerance_percent}%
- Brand: ${design.brand}

## Sections
${sections_table}

## Constraints
${constraints_list}

## Out of Scope
${out_of_scope_list}

## Success Criteria
${success_criteria}
```

### Lessons Block
```
# Lecciones Aprendidas (últimas 5)

${lessons_list}

Formato de cada lección:
- **Síntoma:** Qué se rompió
- **Categoría:** css, build, deploy, auth, visual, etc.
- **Fix:** Qué lo resolvió
- **Proyecto:** Dónde ocurrió
- **Timestamp:** Cuándo se registró
```

---

## 4. INSTRUCTION PROMPTS (Dynamic - 5min TTL)

### Instruction: CSS Fix for Header
```
Fix the header CSS issue:

Current state:
- Header background: ${current_bg}
- Header position: ${current_position}
- Visual diff: ${current_diff}%

Target state (from reference):
- Header background: transparent at top → cream on scroll
- Header position: sticky
- Logo: visible with white filter
- Nav links: visible in both states

Constraint:
${constraint_text}

Steps:
1. Locate: src/components/layout/Header.tsx
2. Find: className with bg-* and position-*
3. Change: Apply scroll-based color transition
4. Test: Verify mobile/tablet/desktop
5. Screenshot: Capture before/after
6. Report: What changed and why

Success: Visual diff decreases and constraint is met
```

### Instruction: Grid Layout Fix
```
Fix the grid layout:

Current state:
- Grid columns: ${current_cols}
- Gap: ${current_gap}
- Visual diff: ${current_diff}%

Target state (from reference):
- Grid columns: ${target_cols}
- Gap: ${target_gap}
- Responsive: ${responsive_spec}

Steps:
1. Locate: src/components/sections/${section_id}.tsx
2. Find: className with grid-cols-*
3. Change: Update grid-cols-* and gap-*
4. Test: Verify at 375px, 768px, 1280px
5. Screenshot: Capture all viewports
6. Report: What changed and why

Success: Visual diff decreases and layout matches reference
```

---

## 5. REPORTING PROMPTS (Dynamic - 5min TTL)

### Report: Section Completed
```
## ✅ Section Completed: ${section_id}

**Status:** DONE
**Visual Diff:** ${final_diff}% (target: < 2%)
**Iterations:** ${iteration_count}
**Time:** ${duration_minutes} minutes

### Changes Made
${changes_list}

### Commits
${commits_list}

### Metrics
- Lighthouse LCP: ${lcp}s
- Lighthouse TBT: ${tbt}ms
- Responsive: ✅ mobile ✅ tablet ✅ desktop

### Next Step
${next_step}
```

### Report: Iteration Failed
```
## ❌ Iteration Failed: ${section_id}

**Status:** STAGNATION (3 iterations without improvement)
**Final Diff:** ${final_diff}%
**Iterations:** ${iteration_count}

### Issues Identified
${issues_list}

### Attempted Fixes
${fixes_list}

### Recommendation
${recommendation}

### Next Step
Escalate to supervisor or manual review required.
```

---

## 6. PROMPT CACHING STRATEGY

### Cache Levels

```
Level 1: System Prompts (1h TTL)
  - Base system prompt
  - Subagent-specific system prompts
  - Shared across all subagents

Level 2: Context Blocks (1h TTL)
  - Methodology
  - Project contract
  - Lessons KB
  - Brand guidelines

Level 3: Task Prompts (30min TTL)
  - Clone site complete
  - Iterate section
  - Fix CSS issue
  - Download assets
  - Validate section

Level 4: Instruction Prompts (5min TTL)
  - Specific fixes
  - Targeted guidance
  - Dynamic based on current state

Level 5: Reporting Prompts (5min TTL)
  - Section completed
  - Iteration failed
  - Deployment status
  - Dynamic based on results
```

### Cache Hit Optimization

```json
{
  "cache_blocks": [
    {
      "name": "system-base",
      "content": "base-system-prompt",
      "ttl_minutes": 60,
      "shared": true,
      "size_tokens": 1200
    },
    {
      "name": "methodology",
      "content": "METODOLOGIA.md",
      "ttl_minutes": 60,
      "shared": true,
      "size_tokens": 2500
    },
    {
      "name": "contract",
      "content": "contracts/${project}.json",
      "ttl_minutes": 60,
      "shared": true,
      "size_tokens": 800
    },
    {
      "name": "lessons",
      "content": "lessons/lessons.jsonl (last 50)",
      "ttl_minutes": 30,
      "shared": true,
      "size_tokens": 3000
    }
  ],
  "total_cached_tokens": 7500,
  "tokens_saved_per_turn": 7500,
  "estimated_savings": "75% of context tokens"
}
```

---

## 7. USAGE EXAMPLES

### Example 1: Clone Site Workflow
```
MCP Supervisor:
  1. Load cache blocks: system-base, methodology, contract
  2. Call reconnaissance with task prompt: "clone-site-complete"
  3. Call content-loader with instruction: "download-assets"
  4. Call layout-builder with task: "iterate-section" (loop)
  5. Call qa-validator with instruction: "validate-section"
  6. Call deployment with instruction: "git-ops-commit"

Cache hits: 4/6 calls (67% cache reuse)
Tokens saved: ~7500 per workflow
```

### Example 2: Iterate Section
```
MCP Supervisor:
  1. Load cache blocks: system-base, contract, lessons
  2. Call qa-validator with instruction: "validate-section"
  3. If diff > tolerance:
     a. Call layout-builder with instruction: "css-fix"
     b. Call deployment with instruction: "git-ops-commit"
     c. Wait 3 min for deploy
     d. Go to step 2

Cache hits: 3/4 calls per iteration (75% cache reuse)
Tokens saved: ~5000 per iteration
```

---

## 8. CACHE INVALIDATION

```
Invalidate cache when:
- Contract changes (version bump)
- Methodology updated
- New lessons added (> 10 new entries)
- Subagent system prompt changes
- Project switches

Manual invalidation:
  mcp_cache_invalidate(block_name)
  mcp_cache_clear_all()

Automatic invalidation:
  - TTL expires
  - File modification detected
  - Version mismatch
```

---

## 9. MONITORING CACHE PERFORMANCE

```json
{
  "metrics": {
    "cache_hit_rate": "78%",
    "tokens_saved_per_day": "45000",
    "cost_reduction": "35%",
    "average_response_time": "2.3s",
    "cache_miss_reasons": [
      "dynamic-instruction-prompts: 15%",
      "reporting-prompts: 5%",
      "contract-updates: 2%"
    ]
  }
}
```

---

## 10. BEST PRACTICES

1. **Reuse system prompts** → Load once, cache for 1h
2. **Batch context blocks** → Methodology + contract + lessons together
3. **Dynamic instructions** → Keep short, specific, 5min TTL
4. **Monitor cache hits** → Aim for > 70% reuse
5. **Invalidate strategically** → Only when necessary
6. **Version prompts** → Track changes in git
7. **Document patterns** → Add to lessons KB
8. **Measure savings** → Track tokens and cost reduction
