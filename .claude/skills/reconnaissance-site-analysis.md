# Skill: reconnaissance:site-analysis

## Purpose
Analyze origin website structure, discover landings, extract specifications, and map content requirements.

## Trigger Patterns
- "analyze site structure"
- "discover landings and content"
- "what's on the origin site"
- MCP calls: `reconnaissance.site-analysis(url)`

## Context You Receive
```json
{
  "url": "https://puebloladehesa.cl",
  "depth": 2,
  "extract_specs": true
}
```

## What You Must Do

### 1. Fetch and Parse Sitemap
- GET `{url}/sitemap.xml`
- Extract all `<loc>` URLs
- Categorize by type (landing, product, utility)

### 2. Discover Landings
- Identify main pages: home, about, contact, products, etc.
- Note URL structure (e.g., `/estadias`, `/experiencias`)
- Screenshot each landing at 1280x1080

### 3. Extract Specifications
- **Brand palette:** Extract colors from CSS/images
- **Typography:** Font families, sizes, weights
- **Layout:** Grid system, spacing, breakpoints
- **Components:** Buttons, cards, forms, navigation

### 4. Map Content vs. Missing
- List all text content found
- List all images/videos with URLs
- Identify missing content (placeholders, TBD sections)
- Create `MISSING_CONTENT.md` with gaps

### 5. Detect Platform
- Check for Shopify liquid tags
- Check for WordPress indicators
- Check for Next.js metadata
- Output: `platform: "shopify" | "wordpress" | "nextjs" | "custom"`

## Output Format

```json
{
  "url": "https://puebloladehesa.cl",
  "platform": "shopify",
  "landings": [
    {
      "name": "home",
      "url": "/",
      "title": "Pueblo La Dehesa",
      "sections": ["header", "hero", "casas-grid", "testimonios", "footer"]
    },
    {
      "name": "estadias",
      "url": "/estadias",
      "title": "Nuestras Casas",
      "sections": ["header", "grid", "filters", "footer"]
    }
  ],
  "brand": {
    "colors": {
      "primary": "#D97757",
      "secondary": "#4A9B8E",
      "bg": "#F5EFE0",
      "text": "#2A2A2A"
    },
    "fonts": {
      "heading": "Serif, font-weight: 300-700",
      "body": "Sans-serif, font-weight: 400"
    }
  },
  "cdn_urls": [
    "https://cdn.shopify.com/s/files/...",
    "https://cdn.shopify.com/s/files/..."
  ],
  "missing_content": [
    "Hero image for 'experiencias' section",
    "Testimonial quotes (3 needed)",
    "FAQ content"
  ],
  "timestamp": "2026-04-26T00:30:00Z"
}
```

## Success Criteria
- ✅ All landings discovered and documented
- ✅ Brand palette extracted with 4+ colors
- ✅ All CDN URLs identified
- ✅ Missing content clearly listed
- ✅ Platform correctly identified
- ✅ Output valid JSON

## Tools You Can Use
- `screenshot` → Capture landings
- `lessons-search` → Find prior analysis patterns

## Constraints
- Do NOT download assets yet (that's content-loader's job)
- Do NOT modify any files
- Do NOT make assumptions about missing content
- Be precise with URLs (case-sensitive)

## Example Success
```
✅ Analyzed https://puebloladehesa.cl
   - 5 landings discovered
   - 47 assets identified
   - Platform: Shopify
   - Missing: 3 testimonials, 1 FAQ section
   - Output: site-recon.json ready for next phase
```
