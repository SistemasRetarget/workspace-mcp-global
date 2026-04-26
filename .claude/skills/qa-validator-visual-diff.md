# Skill: qa-validator:visual-diff

## Purpose
Capture screenshots and compare against reference design to measure visual accuracy.

## Trigger Patterns
- "validate section"
- "compare with reference"
- "how close are we"
- MCP calls: `qa-validator.visual-diff(section, tolerance)`

## Context You Receive
```json
{
  "project": "puebloladehesa-rediseno",
  "section": "casas-grid",
  "deploy_url": "https://puebloladehesa-web-production.up.railway.app",
  "tolerance_percent": 2.0,
  "viewports": ["mobile", "tablet", "desktop"]
}
```

## What You Must Do

### 1. Capture Screenshots
- Screenshot at 375px (mobile)
- Screenshot at 768px (tablet)
- Screenshot at 1280px (desktop)
- Add query param `?screenshot=1` to hide cookie banner
- Wait 1.5s for fonts/images to load
- Save as `actual.png` in evidence directory

### 2. Load Reference
- Find `reference.png` in evidence directory
- Verify it exists, else error
- Note reference viewport size

### 3. Compare Visually
- Use visual-diff.mjs script
- Calculate pixel-level differences
- Generate `diff.png` highlighting changes
- Extract diff percentage

### 4. Analyze Results
- If diff <= tolerance: PASS ✅
- If diff > tolerance: FAIL ❌
- Identify changed regions (header, content, footer)
- Note if changes are acceptable or need fixing

### 5. Generate Report
- Screenshot before/after
- Diff percentage
- Changed areas
- Recommendations

## Output Format

```json
{
  "section": "casas-grid",
  "viewport": "desktop",
  "passed": false,
  "diff_percent": 3.2,
  "tolerance_percent": 2.0,
  "status": "NEEDS_IMPROVEMENT",
  "changed_areas": [
    {
      "region": "grid-cards",
      "change_type": "spacing",
      "severity": "minor"
    },
    {
      "region": "card-shadows",
      "change_type": "styling",
      "severity": "minor"
    }
  ],
  "screenshots": {
    "reference": "evidence/puebloladehesa-rediseno/casas-grid/reference.png",
    "actual": "evidence/puebloladehesa-rediseno/casas-grid/actual.png",
    "diff": "evidence/puebloladehesa-rediseno/casas-grid/diff.png"
  },
  "recommendations": [
    "Adjust card spacing (gap-4 → gap-6)",
    "Increase shadow depth on hover"
  ],
  "timestamp": "2026-04-26T00:40:00Z"
}
```

## Success Criteria
- ✅ Screenshots captured at all viewports
- ✅ Reference found and compared
- ✅ Diff percentage calculated
- ✅ diff.png generated
- ✅ Report includes recommendations
- ✅ Timestamp recorded

## Tools You Can Use
- `screenshot` → Capture URLs
- `visual-diff` → Compare images
- `lessons-search` → Find similar sections

## Constraints
- ✋ Do NOT modify any files
- ✋ Do NOT approve changes (only report)
- ✋ Use consistent viewport sizes
- ✋ Always include `?screenshot=1` param
- ✋ Wait for Railway deploy to be ready

## Interpretation Guide

| Diff % | Status | Action |
|--------|--------|--------|
| 0-1% | ✅ PERFECT | Deploy |
| 1-2% | ✅ GOOD | Deploy |
| 2-5% | ⚠️ ACCEPTABLE | Review, may deploy |
| 5-10% | ❌ NEEDS WORK | Iterate |
| 10%+ | ❌ BLOCKED | Major rework needed |

## Changed Area Types

```
spacing     → Padding, margin, gap differences
colors      → Background, text, border color changes
typography → Font size, weight, line-height changes
layout      → Grid, flex, position changes
shadows     → Box-shadow, text-shadow changes
borders     → Border width, style, color changes
opacity     → Transparency differences
sizing      → Width, height changes
```

## Example Success
```
✅ Validated casas-grid
   Desktop: diff 1.8% (PASS) ✅
   Tablet:  diff 2.1% (PASS) ✅
   Mobile:  diff 1.5% (PASS) ✅
   
   Minor changes detected:
   - Card spacing: 4px wider (acceptable)
   - Shadow depth: slightly more pronounced (acceptable)
   
   Recommendation: READY TO DEPLOY
```

## Failure Example
```
❌ Validated casas-grid
   Desktop: diff 8.2% (FAIL) ❌
   
   Major changes detected:
   - Grid: 2 columns instead of 3
   - Card height: 20% taller
   - Text alignment: centered instead of left
   
   Recommendation: NEEDS LAYOUT FIXES
   Next step: Call layout-builder:css-fix
```
