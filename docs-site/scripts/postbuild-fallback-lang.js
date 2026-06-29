#!/usr/bin/env node
"use strict";

/**
 * Patch known Korean fallback pages after Docusaurus build.
 *
 * ABI v1 has no Korean source files yet; Docusaurus renders the English docs in
 * the ko build but keeps <html lang="ko">. That mis-announces English content to
 * screen readers. The contract for fallback pages is: page lang="en", localized
 * chrome/banner lang="ko", and a visible explanation.
 */

const fs = require("fs");
const path = require("path");

const SITE_DIR = path.join(__dirname, "..");
const BUILD_KO = path.join(SITE_DIR, "build", "ko");
const BANNER_TEXT = "한국어 번역이 아직 없어 영어 원문을 표시합니다.";
const BANNER_HTML = `<div data-maos-fallback-banner="ko" lang="ko" style="border:1px solid #1f6feb;padding:0.75rem;margin:0.75rem 0;background:#f6f8fa;color:#24292f">${BANNER_TEXT}</div>`;
const FALLBACK_SCRIPT = `<script>
(function(){function f(){var h=document.documentElement;h.setAttribute("lang","en");if(!document.querySelector('[data-maos-fallback-banner="ko"]')){var b=document.createElement("div");b.setAttribute("data-maos-fallback-banner","ko");b.setAttribute("lang","ko");b.style.cssText="border:1px solid #1f6feb;padding:0.75rem;margin:0.75rem 0;background:#f6f8fa;color:#24292f";b.textContent="${BANNER_TEXT}";var t=document.body||h;t.insertBefore(b,t.firstChild);}}f();var o=new MutationObserver(function(ms){ms.forEach(function(m){if(m.attributeName==="lang"&&document.documentElement.getAttribute("lang")!=="en")f();});});o.observe(document.documentElement,{attributes:true,attributeFilter:["lang"]});})();
</script>`;

function findMdFiles(dir, baseDir = dir) {
  if (!fs.existsSync(dir)) return [];
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...findMdFiles(full, baseDir));
    } else if ((entry.name.endsWith(".md") || entry.name.endsWith(".mdx")) && !entry.name.startsWith("_related_")) {
      files.push(path.relative(baseDir, full));
    }
  }
  return files;
}

function patch(file) {
  if (!fs.existsSync(file)) return false;
  let html = fs.readFileSync(file, "utf-8");
  html = html.replace(/<html\s+lang="ko"/, '<html lang="en"');
  if (!html.includes('data-maos-fallback-banner="ko"')) {
    html = html.replace(/<body([^>]*)>/, `<body$1>${BANNER_HTML}${FALLBACK_SCRIPT}`);
  } else if (!html.includes("maosAbiFallbackLang")) {
    html = html.replace(/<body([^>]*)>/, `<body$1>${FALLBACK_SCRIPT}`);
  }
  fs.writeFileSync(file, html, "utf-8");
  return true;
}

function main() {
  const docsEn = findMdFiles(path.join(SITE_DIR, "docs"));
  const docsKoDir = path.join(SITE_DIR, "i18n", "ko", "docusaurus-plugin-content-docs", "current");
  const fallbackDocs = docsEn.filter(rel => {
    const base = rel.replace(/\.mdx?$/, "");
    return !fs.existsSync(path.join(docsKoDir, base + ".md")) && !fs.existsSync(path.join(docsKoDir, base + ".mdx"));
  });

  const abiEn = findMdFiles(path.join(SITE_DIR, "abi", "v1"));
  const abiKoDir = path.join(SITE_DIR, "i18n", "ko", "docusaurus-plugin-content-docs-abi", "current");
  const fallbackAbi = abiEn.filter(rel => {
    const base = rel.replace(/\.mdx?$/, "");
    return !fs.existsSync(path.join(abiKoDir, base + ".md")) && !fs.existsSync(path.join(abiKoDir, base + ".mdx"));
  });

  const filesToPatch = [];

  for (const rel of fallbackDocs) {
    const noExt = rel.replace(/\.mdx?$/, "");
    if (path.basename(noExt) === "index") {
      const dir = path.dirname(noExt);
      filesToPatch.push(path.join(BUILD_KO, dir, "index.html"));
    } else {
      filesToPatch.push(path.join(BUILD_KO, noExt, "index.html"));
    }
  }

  for (const rel of fallbackAbi) {
    const noExt = rel.replace(/\.mdx?$/, "");
    if (path.basename(noExt) === "index") {
      const dir = path.dirname(noExt);
      filesToPatch.push(path.join(BUILD_KO, "abi", "v1", dir, "index.html"));
    } else {
      filesToPatch.push(path.join(BUILD_KO, "abi", "v1", noExt, "index.html"));
    }
  }

  const patched = filesToPatch.filter(patch);
  console.log(`postbuild:fallback-lang — patched ${patched.length} ko fallback page(s)`);
}
main();
