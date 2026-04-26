#!/usr/bin/env node
/**
 * visual-diff.mjs — pixel-level UI regression check
 *
 * Usage:
 *   node visual-diff.mjs <reference.png> <actual.png> <diff.png> [tolerance]
 *
 * Output: a single JSON line on stdout, e.g.
 *   {"diff_pixels":1234,"total_pixels":1382400,"diff_percent":0.089,"width":1280,"height":1080,"size_match":true}
 *
 * Exit codes:
 *   0 — diff <= tolerance percent (default 0.5%)
 *   1 — diff exceeds tolerance OR images differ in size
 *   2 — runtime error (file missing, decode failure, etc.)
 */

import fs from "node:fs";
import path from "node:path";
import { PNG } from "pngjs";
import pixelmatch from "pixelmatch";

const [, , refPath, actualPath, diffPath, toleranceArg] = process.argv;
const tolerance = parseFloat(toleranceArg ?? "0.5"); // percent

if (!refPath || !actualPath || !diffPath) {
  console.error("usage: visual-diff.mjs <ref.png> <actual.png> <diff.png> [tolerance%]");
  process.exit(2);
}

function loadPng(p) {
  if (!fs.existsSync(p)) {
    console.error(`missing file: ${p}`);
    process.exit(2);
  }
  return PNG.sync.read(fs.readFileSync(p));
}

const ref = loadPng(refPath);
const actual = loadPng(actualPath);

if (ref.width !== actual.width || ref.height !== actual.height) {
  // Size mismatch is an instant fail — we can't pixelmatch different sizes.
  const out = {
    diff_pixels: -1,
    total_pixels: ref.width * ref.height,
    diff_percent: 100,
    width: ref.width,
    height: ref.height,
    actual_width: actual.width,
    actual_height: actual.height,
    size_match: false,
    tolerance_percent: tolerance,
    passed: false,
    reason: "viewport/size mismatch",
  };
  console.log(JSON.stringify(out));
  process.exit(1);
}

const { width, height } = ref;
const diff = new PNG({ width, height });
const diffPixels = pixelmatch(ref.data, actual.data, diff.data, width, height, {
  threshold: 0.1,
  alpha: 0.3,
  diffColor: [255, 0, 0],
});

fs.mkdirSync(path.dirname(diffPath), { recursive: true });
fs.writeFileSync(diffPath, PNG.sync.write(diff));

const total = width * height;
const percent = (diffPixels / total) * 100;
const passed = percent <= tolerance;

const result = {
  diff_pixels: diffPixels,
  total_pixels: total,
  diff_percent: Number(percent.toFixed(4)),
  width,
  height,
  size_match: true,
  tolerance_percent: tolerance,
  passed,
  diff_image: diffPath,
};

console.log(JSON.stringify(result));
process.exit(passed ? 0 : 1);
