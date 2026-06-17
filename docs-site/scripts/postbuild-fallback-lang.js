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

const BUILD_KO_ABI = path.join(__dirname, "..", "build", "ko", "abi", "v1");
const BANNER_TEXT = "한국어 번역이 아직 없어 영어 원문을 표시합니다.";
const BANNER_HTML = `<div data-maos-fallback-banner="ko" lang="ko" style="border:1px solid #1f6feb;padding:0.75rem;margin:0.75rem 0;background:#f6f8fa;color:#24292f">${BANNER_TEXT}</div>`;
const FALLBACK_SCRIPT = `<script>
(function(){function f(){var h=document.documentElement;h.setAttribute("lang","en");if(!document.querySelector('[data-maos-fallback-banner="ko"]')){var b=document.createElement("div");b.setAttribute("data-maos-fallback-banner","ko");b.setAttribute("lang","ko");b.style.cssText="border:1px solid #1f6feb;padding:0.75rem;margin:0.75rem 0;background:#f6f8fa;color:#24292f";b.textContent="${BANNER_TEXT}";var t=document.body||h;t.insertBefore(b,t.firstChild);}}f();var o=new MutationObserver(function(ms){ms.forEach(function(m){if(m.attributeName==="lang"&&document.documentElement.getAttribute("lang")!=="en")f();});});o.observe(document.documentElement,{attributes:true,attributeFilter:["lang"]});})();
</script>`;

function htmlFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...htmlFiles(full));
    else if (entry.name.endsWith(".html")) files.push(full);
  }
  return files;
}

function patch(file) {
  let html = fs.readFileSync(file, "utf-8");
  // Statically set the page language to English and insert the banner/script.
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
  const patched = htmlFiles(BUILD_KO_ABI).filter(patch);
  console.log(`postbuild:fallback-lang — patched ${patched.length} ko ABI fallback page(s)`);
}

main();
