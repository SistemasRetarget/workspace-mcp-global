# Skill: layout-builder:css-fix

## Purpose
Fix CSS/Tailwind issues in a section to match reference design.

## Trigger Patterns
- "header is not transparent"
- "grid should be 3 columns on desktop"
- "spacing is wrong"
- MCP calls: `layout-builder.css-fix(section, issue, constraint)`

## Context You Receive
```json
{
  "section": "hero-banner",
  "issue": "header is not transparent over image",
  "constraint": "sticky: transparent at top → cream on scroll (NO black)",
  "reference_image": "/path/to/reference.png",
  "current_diff_percent": 45.2
}
```

## What You Must Do

### 1. Analyze the Issue
- Read the constraint carefully
- Understand what "correct" looks like from reference
- Identify which CSS classes need changes
- Check Tailwind config for available utilities

### 2. Locate the File
- Find the component file (usually in `src/components/` or `src/app/`)
- Identify the exact CSS classes or style blocks
- Note current values

### 3. Apply Minimal Fix
- Change ONLY what's necessary
- Use Tailwind utilities when possible
- Preserve responsive design (mobile/tablet/desktop)
- Keep component structure unchanged

### 4. Test Responsiveness
- Verify mobile (375px) still works
- Verify tablet (768px) still works
- Verify desktop (1280px) matches reference

### 5. Verify Constraints
- Check all constraints are met
- Ensure no new issues introduced
- Confirm brand colors used correctly

## Output Format

```json
{
  "section": "hero-banner",
  "file": "src/components/layout/Header.tsx",
  "changes": [
    {
      "line": 42,
      "old": "bg-black/50",
      "new": "bg-transparent",
      "reason": "Header must be transparent at top"
    },
    {
      "line": 45,
      "old": "text-white",
      "new": "text-brand-ink group-[.scrolled]:text-brand-ink",
      "reason": "Text color changes on scroll"
    }
  ],
  "files_modified": ["src/components/layout/Header.tsx"],
  "screenshot_before": "/path/to/before.png",
  "screenshot_after": "/path/to/after.png",
  "diff_percent_before": 45.2,
  "diff_percent_after": 28.5,
  "status": "IMPROVED"
}
```

## Success Criteria
- ✅ Issue resolved (visual diff decreased)
- ✅ Responsive design maintained
- ✅ Constraints satisfied
- ✅ No new issues introduced
- ✅ Minimal changes (< 10 lines)
- ✅ Code style consistent

## Tools You Can Use
- `edit` → Modify files
- `screenshot` → Capture before/after
- `lessons-search` → Find similar fixes

## Constraints
- ✋ Do NOT add new dependencies
- ✋ Do NOT change component structure
- ✋ Do NOT modify unrelated sections
- ✋ Respect existing responsive breakpoints
- ✋ Use brand colors from tailwind.config.ts

## Common Patterns

### Transparent Header Over Image
```tsx
// BEFORE
<header className="bg-black/50 sticky top-0">

// AFTER
<header className="bg-transparent sticky top-0 transition-colors duration-300 group-[.scrolled]:bg-brand-bg">
```

### Grid Responsive
```tsx
// BEFORE
<div className="grid grid-cols-2 gap-4">

// AFTER
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
```

### Sticky with Scroll Detection
```tsx
// Use scroll listener to toggle class
useEffect(() => {
  const handleScroll = () => {
    if (window.scrollY > 0) {
      document.documentElement.classList.add('scrolled');
    } else {
      document.documentElement.classList.remove('scrolled');
    }
  };
  window.addEventListener('scroll', handleScroll);
  return () => window.removeEventListener('scroll', handleScroll);
}, []);
```

## Example Success
```
✅ Fixed hero-banner header
   - Changed: bg-black/50 → bg-transparent
   - Added: scroll-based color transition
   - Result: diff 45.2% → 28.5%
   - Responsive: ✅ mobile ✅ tablet ✅ desktop
```
