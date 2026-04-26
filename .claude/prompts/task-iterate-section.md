# Task Prompt: Iterate Section

## Objective
Iterate a single section until visual diff is acceptable (< tolerance_percent).

## Input Parameters

```json
{
  "section_id": "casas-grid",
  "deploy_url": "https://puebloladehesa-web-production.up.railway.app",
  "tolerance_percent": 2.0,
  "max_iterations": 4,
  "current_diff_percent": 8.5
}
```

## Iteration Loop

### Step 1: Capture Screenshot
**Subagent:** qa-validator  
**Skill:** screenshot-capture

Action:
- Capture at 1280x1080 (desktop)
- Add query param: `?screenshot=1`
- Wait 1.5s for load
- Save as `actual.png`

Output: Screenshot file

### Step 2: Compare with Reference
**Subagent:** qa-validator  
**Skill:** visual-diff

Action:
- Load reference.png
- Compare with actual.png
- Calculate diff percentage
- Generate diff.png
- Identify changed areas

Output:
```json
{
  "diff_percent": 3.2,
  "passed": false,
  "changed_areas": [
    {"region": "grid-cards", "severity": "minor"},
    {"region": "spacing", "severity": "minor"}
  ]
}
```

### Step 3: Check Convergence

**If diff <= tolerance:**
- ✅ DONE
- Mark section as DONE
- Report success
- Move to next section

**If diff > tolerance:**
- Continue to Step 4

### Step 4: Identify Issues
**Subagent:** qa-validator

Analyze diff.png and identify:
- Spacing issues
- Color differences
- Layout problems
- Typography changes

Output: Issue list with recommendations

### Step 5: Request CSS Fix
**Subagent:** layout-builder  
**Skill:** css-fix

Input:
```json
{
  "section": "casas-grid",
  "issue": "Grid is 2 columns, should be 3",
  "constraint": "3 columns on desktop, 2 on tablet, 1 on mobile",
  "current_diff": 3.2
}
```

Action:
- Locate file
- Apply minimal fix
- Test responsiveness
- Capture before/after

Output: Modified files

### Step 6: Commit and Push
**Subagent:** deployment  
**Skill:** git-ops

Action:
- Stage modified files
- Commit with message
- Push to main
- Trigger Railway build

Output: Commit hash

### Step 7: Wait for Deploy
- Wait 3 minutes for Railway build
- Verify deploy succeeded
- Confirm URL is live

### Step 8: Loop Back
- Go to Step 1
- Capture new screenshot
- Compare again
- Check if converged

## Loop Control

```
Iteration 1: diff 8.5% → 6.2% (improved)
Iteration 2: diff 6.2% → 4.1% (improved)
Iteration 3: diff 4.1% → 2.8% (improved)
Iteration 4: diff 2.8% → 1.8% (PASS ✅)

Total iterations: 4
Status: CONVERGED
```

## Stagnation Detection

If 3 consecutive iterations show NO improvement:
- Stop iterating
- Report STAGNATION
- Escalate to supervisor
- Request manual review

## Success Criteria

- ✅ Visual diff <= tolerance_percent
- ✅ Responsive design maintained
- ✅ No new issues introduced
- ✅ Minimal commits (1 per iteration)
- ✅ Converged within max_iterations

## Failure Criteria

- ❌ Stagnation (3 iterations no improvement)
- ❌ Exceeded max_iterations
- ❌ New issues introduced
- ❌ Responsive design broken

## Reporting

```
## Iterate Section: ${section_id}

**Status:** ✅ CONVERGED
**Iterations:** 4
**Initial Diff:** 8.5%
**Final Diff:** 1.8%
**Improvement:** 6.7%

### Iteration Details
| Iter | Diff % | Change | Commit |
|------|--------|--------|--------|
| 1 | 8.5% → 6.2% | -2.3% | abc123 |
| 2 | 6.2% → 4.1% | -2.1% | def456 |
| 3 | 4.1% → 2.8% | -1.3% | ghi789 |
| 4 | 2.8% → 1.8% | -1.0% | jkl012 |

### Changes Made
- Grid columns: 2 → 3 (desktop)
- Gap: 4 → 6 (spacing)
- Card height: adjusted for content

### Responsive Status
- ✅ Mobile (375px)
- ✅ Tablet (768px)
- ✅ Desktop (1280px)

### Next Step
- Move to next section
- Or deploy if all sections done
```

## Constraints

- Do NOT exceed max_iterations
- Do NOT skip validation steps
- Do NOT modify unrelated sections
- Do NOT commit unrelated changes
- Do NOT force push
