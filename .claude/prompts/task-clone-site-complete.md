# Task Prompt: Clone Site Complete

## Objective
Clone a website from origin to new codebase, following the 7-phase methodology.

## Input Parameters

```json
{
  "origin_url": "https://puebloladehesa.cl",
  "project_name": "puebloladehesa-rediseno",
  "evidence_dir": "~/Documents/workspace-mcp-global/evidence/puebloladehesa-rediseno",
  "contract_file": "contracts/puebloladehesa-rediseno.json",
  "max_iterations": 4,
  "tolerance_percent": 2.0
}
```

## Workflow Phases

### Phase 1: Reconnaissance
**Subagent:** reconnaissance  
**Skill:** site-analysis

Input: Origin URL  
Output: site-recon.json with:
- All landings discovered
- Brand palette (colors, fonts)
- CDN URLs
- Platform identification
- Missing content list

Success: All landings documented, brand extracted, CDN URLs identified

### Phase 2: Asset Download
**Subagent:** content-loader  
**Skill:** asset-download

Input: CDN URLs from site-recon.json  
Output: assets-manifest.json with:
- Downloaded count
- Failed count
- Local paths
- Optimization results

Success: All accessible assets downloaded, manifest generated

### Phase 3: Layout Build
**Subagent:** layout-builder  
**Skill:** component-build

Input: Sections from site-recon.json  
Output: src/components/sections/*.tsx

Loop until all sections done:
1. Create component
2. Capture screenshot
3. Compare with reference
4. If diff > 2%: iterate with css-fix
5. If diff <= 2%: mark DONE

Success: All sections built, diff <= 2% per section

### Phase 4: Visual Validation
**Subagent:** qa-validator  
**Skill:** visual-diff

Input: All sections  
Output: validation-report.json

For each section:
1. Capture at mobile/tablet/desktop
2. Compare with reference
3. If diff > 2%: feedback to layout-builder
4. Loop max 4 times

Success: All sections pass (diff <= 2%)

### Phase 5: Content Loading
**Subagent:** layout-builder  
**Skill:** content-load

Input: Approved content  
Output: Updated components with content

Success: All text, images, links loaded

### Phase 6: Interactions + Colors
**Subagent:** layout-builder  
**Skill:** interactions-apply

Input: Interaction specs from site-recon  
Output: Components with hover/scroll effects

Success: All interactions working, brand colors enforced

### Phase 7: Deployment
**Subagent:** deployment  
**Skill:** git-ops + railway-deploy

Input: All modified files  
Output: Deployed to production

Steps:
1. Commit all changes
2. Push to main
3. Wait for Railway build
4. Smoke test
5. Verify live

Success: Site live and functional

## Success Criteria

- ✅ All sections built
- ✅ Visual diff < 2% per section
- ✅ All assets downloaded
- ✅ Lighthouse LCP < 2.5s
- ✅ npm audit: 0 critical
- ✅ Responsive: mobile/tablet/desktop
- ✅ Brand colors enforced
- ✅ Ready for production

## Reporting

After completion, report:

```
## Clone Site Complete: ${project_name}

**Status:** ✅ DONE
**Duration:** ${total_minutes} minutes
**Sections:** ${sections_count}
**Iterations:** ${total_iterations}
**Final Diff:** ${avg_final_diff}%

### Sections Completed
${sections_table}

### Assets
- Downloaded: ${asset_count}
- Total size: ${total_size_mb}MB
- Optimized: ${optimization_ratio}

### Commits
${commits_list}

### Metrics
- Lighthouse LCP: ${lcp}s
- Lighthouse TBT: ${tbt}ms
- npm audit: ${audit_status}

### Deploy
- URL: ${deploy_url}
- Status: ✅ Live
- Build time: ${build_time}s

### Next Steps
- Monitor for issues
- Gather user feedback
- Plan iterations
```

## Constraints

- Do NOT skip phases
- Do NOT exceed max_iterations per section
- Do NOT modify contract
- Do NOT add out-of-scope features
- Do NOT commit secrets
