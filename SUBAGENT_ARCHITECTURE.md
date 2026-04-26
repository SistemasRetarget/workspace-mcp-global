# Arquitectura de Subagentes para MCP Supervisor

**Objetivo:** Orquestar trabajo distribuido entre subagentes especializados, coordinados por el MCP supervisor.

---

## Modelo de Orquestación

```
Usuario (Luis)
    ↓
MCP Supervisor (orchestrator)
    ├─→ Subagente: Reconnaissance
    │   └─ Skills: site-analysis, asset-discovery, content-mapping
    │
    ├─→ Subagente: Layout Builder
    │   └─ Skills: css-fix, responsive-design, visual-alignment
    │
    ├─→ Subagente: Content Loader
    │   └─ Skills: asset-download, cdn-management, image-optimization
    │
    ├─→ Subagente: QA Validator
    │   └─ Skills: screenshot-capture, visual-diff, lighthouse-audit
    │
    └─→ Subagente: Deployment
        └─ Skills: git-ops, railway-deploy, smoke-test
```

---

## Subagentes Especializados

### 1. **Reconnaissance Subagent**
**Rol:** Analizar sitio origen y extraer especificaciones

**Skills:**
- `site-analysis` → Fetch sitemap.xml, descubre landings, estructura
- `asset-discovery` → Extrae URLs de CDN, identifica recursos
- `content-mapping` → Mapea contenido vs. lo que falta
- `platform-detect` → Identifica plataforma (Shopify, WordPress, Next.js)

**Input:** URL del sitio origen  
**Output:** `site-recon.json` + `MISSING_CONTENT.md`

**Ejemplo de uso:**
```json
{
  "subagent": "reconnaissance",
  "skill": "site-analysis",
  "input": {
    "url": "https://puebloladehesa.cl",
    "depth": 2
  }
}
```

---

### 2. **Layout Builder Subagent**
**Rol:** Construir y ajustar layouts CSS/HTML

**Skills:**
- `css-fix` → Ajusta estilos Tailwind/CSS
- `responsive-design` → Adapta para mobile/tablet/desktop
- `visual-alignment` → Alinea elementos con referencia
- `component-build` → Crea componentes React/Next.js

**Input:** Especificaciones de sección + referencia visual  
**Output:** Código HTML/CSS + screenshot

**Ejemplo de uso:**
```json
{
  "subagent": "layout-builder",
  "skill": "css-fix",
  "input": {
    "section": "hero-banner",
    "issue": "header no está transparente sobre imagen",
    "constraint": "sticky: transparent arriba → crema al scroll"
  }
}
```

---

### 3. **Content Loader Subagent**
**Rol:** Descargar y gestionar assets

**Skills:**
- `asset-download` → Descarga imágenes/videos de CDN
- `cdn-management` → Organiza assets en `public/media/`
- `image-optimization` → Comprime y convierte a WebP
- `metadata-extract` → Extrae EXIF, dimensiones, etc.

**Input:** Lista de URLs de assets  
**Output:** Assets descargados + manifest.json

**Ejemplo de uso:**
```json
{
  "subagent": "content-loader",
  "skill": "asset-download",
  "input": {
    "urls": [
      "https://cdn.example.com/image1.jpg",
      "https://cdn.example.com/image2.jpg"
    ],
    "target_dir": "public/media/casas"
  }
}
```

---

### 4. **QA Validator Subagent**
**Rol:** Validar visualmente y auditar

**Skills:**
- `screenshot-capture` → Captura URLs con Playwright
- `visual-diff` → Compara actual vs. referencia
- `lighthouse-audit` → Mide performance (LCP, TBT, CLS)
- `accessibility-check` → Valida WCAG 2.1

**Input:** URL a validar + tolerancia visual  
**Output:** Screenshots + diff.png + reporte

**Ejemplo de uso:**
```json
{
  "subagent": "qa-validator",
  "skill": "visual-diff",
  "input": {
    "project": "puebloladehesa-rediseno",
    "section": "casas-grid",
    "tolerance_percent": 2.0
  }
}
```

---

### 5. **Deployment Subagent**
**Rol:** Gestionar git, commits y deploys

**Skills:**
- `git-ops` → Commit, push, branch management
- `railway-deploy` → Trigger deploy en Railway
- `smoke-test` → Valida que el deploy fue exitoso
- `rollback` → Revierte a commit anterior si falla

**Input:** Cambios a commitear + mensaje  
**Output:** Commit hash + deploy status

**Ejemplo de uso:**
```json
{
  "subagent": "deployment",
  "skill": "git-ops",
  "input": {
    "action": "commit",
    "message": "fix(hero): adjust header transparency",
    "files": ["src/components/layout/Header.tsx"]
  }
}
```

---

## Flujo de Trabajo: Clone-Site Completo

### Fase 1: Reconnaissance (Subagent 1)
```
MCP → reconnaissance.site-analysis(url)
  ↓
  Descubre: 5 landings, 47 assets, plataforma Shopify
  ↓
  Output: site-recon.json
```

### Fase 2: Asset Download (Subagent 3)
```
MCP → content-loader.asset-download(urls)
  ↓
  Descarga 47 imágenes a public/media/
  ↓
  Output: assets-manifest.json
```

### Fase 3: Layout Build (Subagent 2)
```
MCP → layout-builder.component-build(specs)
  ↓
  Crea componentes React para cada sección
  ↓
  Output: src/components/sections/*.tsx
```

### Fase 4: Visual Validation (Subagent 4)
```
MCP → qa-validator.visual-diff(section)
  ↓
  Captura screenshot, compara con referencia
  ↓
  Si diff > 2%:
    → MCP solicita ajuste a layout-builder
    → Vuelve a validar
  Si diff <= 2%:
    → Sección DONE
```

### Fase 5: Deploy (Subagent 5)
```
MCP → deployment.git-ops(commit)
  ↓
  Commit + push a main
  ↓
  MCP → deployment.railway-deploy()
  ↓
  Espera 3 min, valida con smoke-test
  ↓
  Output: deploy status + URL
```

---

## Coordinación: MCP como Orquestador

El MCP supervisor **NO ejecuta trabajo**, solo **orquesta**:

```rust
// Pseudocódigo en main.rs

fn orchestrate_clone_site(url: &str) {
    // 1. Reconnaissance
    let recon = call_subagent("reconnaissance", "site-analysis", {
        "url": url
    });
    
    // 2. Asset Download
    let assets = call_subagent("content-loader", "asset-download", {
        "urls": recon.cdn_urls
    });
    
    // 3. Layout Build (loop hasta converger)
    for section in recon.sections {
        loop {
            let layout = call_subagent("layout-builder", "component-build", {
                "section": section,
                "reference": section.reference_image
            });
            
            // 4. Validate
            let diff = call_subagent("qa-validator", "visual-diff", {
                "section": section
            });
            
            if diff.percent <= 2.0 {
                break; // Sección OK
            }
            
            // Feedback al layout-builder
            layout = call_subagent("layout-builder", "css-fix", {
                "section": section,
                "issue": diff.issues
            });
        }
    }
    
    // 5. Deploy
    call_subagent("deployment", "git-ops", {
        "action": "commit",
        "message": "feat: clone site complete"
    });
    
    call_subagent("deployment", "railway-deploy", {});
}
```

---

## Skills vs. Tools vs. Subagents

| Concepto | Dónde vive | Quién lo usa | Ejemplo |
|----------|-----------|-------------|---------|
| **Skill** | `.claude/skills/` | Claude (subagent) | `layout-builder:css-fix` |
| **Tool** | MCP (Rust) | Subagent (via MCP) | `screenshot`, `visual-diff` |
| **Subagent** | Claude instance | MCP supervisor | `reconnaissance`, `layout-builder` |
| **MCP** | Rust server | Todos los subagents | Orquestador central |

---

## Implementación: Skill Files

Cada subagente tiene un skill file en `.claude/skills/`:

### `layout-builder-css-fix.md`
```markdown
# Skill: layout-builder:css-fix

## Trigger
- Usuario dice: "el header no está transparente"
- MCP dice: "css-fix needed for hero section"

## Context
- Recibe: sección, issue, constraint
- Debe: ajustar CSS/Tailwind
- Output: archivo editado + screenshot

## Constraints
- No agregar dependencias nuevas
- Mantener responsive design
- Respetar brand colors

## Success Criteria
- Visual diff < 2%
- Lighthouse LCP < 2.5s
```

### `qa-validator-visual-diff.md`
```markdown
# Skill: qa-validator:visual-diff

## Trigger
- MCP dice: "validate section"
- Usuario dice: "compara con referencia"

## Context
- Captura screenshot de deploy
- Compara con reference.png
- Devuelve diff % y diff.png

## Success Criteria
- diff_percent <= tolerance
- diff.png generado
```

---

## Configuración en MCP Config

```json
{
  "subagents": [
    {
      "name": "reconnaissance",
      "model": "claude-opus",
      "skills": ["site-analysis", "asset-discovery", "content-mapping", "platform-detect"],
      "tools": ["screenshot", "lessons-search"],
      "context_cache": true
    },
    {
      "name": "layout-builder",
      "model": "claude-opus",
      "skills": ["css-fix", "responsive-design", "visual-alignment", "component-build"],
      "tools": ["edit", "write", "screenshot"],
      "context_cache": true
    },
    {
      "name": "content-loader",
      "model": "claude-opus",
      "skills": ["asset-download", "cdn-management", "image-optimization"],
      "tools": ["bash", "write"],
      "context_cache": true
    },
    {
      "name": "qa-validator",
      "model": "claude-opus",
      "skills": ["screenshot-capture", "visual-diff", "lighthouse-audit", "accessibility-check"],
      "tools": ["screenshot", "visual-diff", "validate"],
      "context_cache": true
    },
    {
      "name": "deployment",
      "model": "claude-opus",
      "skills": ["git-ops", "railway-deploy", "smoke-test", "rollback"],
      "tools": ["bash", "railway-logs"],
      "context_cache": true
    }
  ]
}
```

---

## Ventajas de Esta Arquitectura

| Aspecto | Beneficio |
|--------|-----------|
| **Especialización** | Cada subagent experto en su dominio |
| **Paralelización** | Reconnaissance + asset-download en paralelo |
| **Reutilización** | Skills se reutilizan entre proyectos |
| **Escalabilidad** | Agregar nuevos subagents sin cambiar MCP |
| **Debugging** | Cada subagent tiene su propio contexto |
| **Caching** | Prompt caching por subagent |
| **Fallback** | Si un subagent falla, MCP lo reintentar |

---

## Próximos Pasos

1. **Crear skill files** en `.claude/skills/` para cada subagent
2. **Actualizar MCP** para soportar `call_subagent(name, skill, input)`
3. **Configurar context caching** por subagent
4. **Probar flujo completo** en puebloladehesa-rediseno
5. **Documentar runbooks** para cada subagent

---

## Ejemplo Real: Iterar casas-grid

```
Usuario: "arregla casas-grid, debe ser 3 columnas en desktop"

MCP Supervisor:
  1. Llama qa-validator.screenshot-capture()
     → Captura estado actual
  
  2. Llama qa-validator.visual-diff()
     → Compara con referencia
     → Resultado: "grid es 2 columnas, debe ser 3"
  
  3. Llama layout-builder.css-fix()
     → Input: "change grid from 2 cols to 3 cols"
     → Output: archivo editado
  
  4. Llama qa-validator.screenshot-capture()
     → Captura nuevo estado
  
  5. Llama qa-validator.visual-diff()
     → Si diff <= 2%: DONE
     → Si diff > 2%: vuelve a paso 3
  
  6. Llama deployment.git-ops()
     → Commit + push
  
  7. Llama deployment.railway-deploy()
     → Espera build, valida smoke-test

Usuario recibe: "casas-grid DONE (3 iteraciones, diff 1.2%)"
```

---

## Conclusión

El MCP supervisor + subagentes especializados + skills = **arquitectura escalable y mantenible** para desarrollo autónomo de sitios web.

Cada componente tiene una responsabilidad clara, y el MCP orquesta el flujo sin ejecutar trabajo directamente.
