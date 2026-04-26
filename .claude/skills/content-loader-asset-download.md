# Skill: content-loader:asset-download

## Purpose
Download images, videos, and other assets from CDN to local project directory.

## Trigger Patterns
- "download all images"
- "fetch assets from CDN"
- "save images locally"
- MCP calls: `content-loader.asset-download(urls, target_dir)`

## Context You Receive
```json
{
  "urls": [
    "https://cdn.shopify.com/s/files/1/0123/4567/8901/products/image1.jpg",
    "https://cdn.shopify.com/s/files/1/0123/4567/8901/products/image2.jpg"
  ],
  "target_dir": "public/media/casas",
  "optimize": true,
  "formats": ["webp", "jpg"]
}
```

## What You Must Do

### 1. Validate URLs
- Check all URLs are accessible (HEAD request)
- Verify HTTPS
- Note any redirects
- Skip broken URLs, log them

### 2. Download Assets
- Create target directory if missing
- Download each file
- Preserve original filename or use semantic names
- Track download progress

### 3. Optimize Images (if enabled)
- Convert to WebP for modern browsers
- Keep JPG fallback
- Compress without quality loss
- Extract dimensions and EXIF data

### 4. Generate Manifest
- Create `assets-manifest.json` with:
  - Original URL → local path mapping
  - File size, dimensions, format
  - Download timestamp
  - Checksum for integrity

### 5. Update References (if needed)
- Search codebase for CDN URLs
- Replace with local paths
- Update `next/image` imports

## Output Format

```json
{
  "downloaded": 47,
  "failed": 2,
  "total_size_mb": 23.4,
  "target_dir": "public/media/casas",
  "manifest": {
    "https://cdn.shopify.com/s/files/.../image1.jpg": {
      "local_path": "public/media/casas/casa-1-exterior.jpg",
      "local_path_webp": "public/media/casas/casa-1-exterior.webp",
      "original_size_kb": 245,
      "optimized_size_kb": 89,
      "dimensions": "1920x1080",
      "format": "jpg",
      "downloaded_at": "2026-04-26T00:35:00Z"
    }
  },
  "failed_urls": [
    {
      "url": "https://cdn.example.com/missing.jpg",
      "reason": "404 Not Found"
    }
  ],
  "status": "COMPLETED"
}
```

## Success Criteria
- ✅ All accessible URLs downloaded
- ✅ Files saved to correct directory
- ✅ Images optimized (if enabled)
- ✅ Manifest generated
- ✅ Failed URLs logged
- ✅ Total size < 100MB (warn if larger)

## Tools You Can Use
- `bash` → Download with curl/wget
- `write` → Create manifest.json
- `lessons-search` → Find optimization patterns

## Constraints
- ✋ Do NOT modify original files
- ✋ Do NOT overwrite existing assets without confirmation
- ✋ Respect rate limits (add delays between downloads)
- ✋ Do NOT download if file already exists (check hash)
- ✋ Keep directory structure clean

## Download Strategy

```bash
# For each URL:
1. HEAD request to check size/type
2. If image: download
3. If video: download (warn if > 50MB)
4. If other: download
5. Verify checksum
6. Optimize if image
7. Log to manifest
```

## Optimization Rules

```
JPG/PNG → WebP (80% quality)
  - Original: image.jpg (245KB)
  - WebP: image.webp (89KB)
  - Fallback: image.jpg still available

Video → Keep original
  - No conversion
  - Just organize in public/media/videos/

SVG → Keep original
  - No optimization needed
```

## Example Success
```
✅ Downloaded 47 assets
   - 45 images (23.4 MB → 8.7 MB optimized)
   - 2 videos (skipped optimization)
   - Manifest: assets-manifest.json
   - Failed: 2 URLs (logged in manifest)
```

## Manifest Example
```json
{
  "version": 1,
  "generated_at": "2026-04-26T00:35:00Z",
  "assets": {
    "https://cdn.shopify.com/s/files/.../casa-1.jpg": {
      "local": "public/media/casas/casa-1.jpg",
      "webp": "public/media/casas/casa-1.webp",
      "size_original_kb": 245,
      "size_optimized_kb": 89,
      "width": 1920,
      "height": 1080,
      "format": "jpg"
    }
  },
  "stats": {
    "total_downloaded": 47,
    "total_failed": 2,
    "total_size_mb": 23.4,
    "compression_ratio": 0.37
  }
}
```
