# Skill: deployment:git-ops

## Purpose
Manage git operations: commit, push, branch management, and deployment coordination.

## Trigger Patterns
- "commit these changes"
- "push to main"
- "create a branch"
- MCP calls: `deployment.git-ops(action, message, files)`

## Context You Receive
```json
{
  "action": "commit",
  "message": "fix(hero): adjust header transparency and negative margin",
  "files": [
    "src/components/layout/Header.tsx",
    "src/app/(frontend)/(es)/page.tsx"
  ],
  "branch": "main",
  "auto_push": true
}
```

## What You Must Do

### 1. Validate Git State
- Check git status
- Verify no uncommitted changes (except target files)
- Confirm branch is correct
- Verify remote is reachable

### 2. Stage Files
- `git add` only specified files
- Verify staging with `git status`
- Do NOT stage unrelated files

### 3. Create Commit
- Use provided message
- Follow conventional commits: `type(scope): description`
- Include issue reference if applicable
- Keep message concise (< 72 chars first line)

### 4. Push to Remote
- `git push origin <branch>`
- Verify push succeeded
- Note commit hash

### 5. Verify Deployment Trigger
- Check if Railway auto-deploy is enabled
- Note deploy URL
- Estimate build time (usually 2-3 min)

## Output Format

```json
{
  "action": "commit",
  "status": "SUCCESS",
  "commit_hash": "f46e326abc123def456789",
  "commit_message": "fix(hero): adjust header transparency and negative margin",
  "files_committed": 2,
  "branch": "main",
  "push_status": "SUCCESS",
  "deploy_triggered": true,
  "deploy_url": "https://puebloladehesa-web-production.up.railway.app",
  "estimated_build_time_seconds": 180,
  "timestamp": "2026-04-26T00:45:00Z"
}
```

## Success Criteria
- ✅ All specified files committed
- ✅ Commit message follows convention
- ✅ Push succeeded to remote
- ✅ Commit hash recorded
- ✅ Deploy triggered (if auto-deploy enabled)
- ✅ No unrelated files staged

## Tools You Can Use
- `bash` → Run git commands
- `logs` → Check git log
- `railway-logs` → Monitor deploy

## Constraints
- ✋ Do NOT commit unrelated changes
- ✋ Do NOT force push to main
- ✋ Do NOT delete branches without confirmation
- ✋ Do NOT commit secrets or API keys
- ✋ Do NOT commit node_modules or build artifacts

## Git Workflow

```bash
# 1. Check status
git status

# 2. Stage specific files
git add src/components/layout/Header.tsx
git add src/app/(frontend)/(es)/page.tsx

# 3. Verify staging
git status

# 4. Create commit
git commit -m "fix(hero): adjust header transparency and negative margin"

# 5. Push
git push origin main

# 6. Verify
git log -1 --oneline
```

## Commit Message Convention

```
type(scope): description

[optional body]
[optional footer]

Types: feat, fix, docs, style, refactor, perf, test, chore
Scope: component or section affected
Description: what changed and why
```

### Examples
```
✅ fix(hero): adjust header transparency and negative margin
✅ feat(casas-grid): change from 2 to 3 columns on desktop
✅ style(header): update scroll-based color transition
❌ fixed stuff
❌ update
```

## Branch Strategy

```
main          → Production-ready code
  ↓
feature/*     → Feature branches (if needed)
hotfix/*      → Emergency fixes
```

## Deploy Coordination

After push:
1. Railway auto-detects push to main
2. Starts build (2-3 min typical)
3. Deploys to production
4. Health check runs
5. Available at deploy_url

## Example Success
```
✅ Committed and pushed
   - Commit: f46e326
   - Message: fix(hero): adjust header transparency
   - Files: 2
   - Branch: main
   - Deploy: triggered
   - ETA: 3 minutes
```

## Rollback Procedure

If deploy fails:
```bash
git revert <commit_hash>
git push origin main
# Railway redeploys automatically
```

## Pre-Commit Checklist

Before committing:
- [ ] All changes are intentional
- [ ] No console.log() or debug code
- [ ] No secrets in code
- [ ] Tests pass (if applicable)
- [ ] Code follows project style
- [ ] Commit message is clear
