import fs from "node:fs";
import path from "node:path";

export type ManifestRoute = { path: string; label: string; content_floor?: string };
export type ManifestRedirect = { from: string; to: string };
export type RouteManifest = {
  routes: ManifestRoute[];
  redirects?: ManifestRedirect[];
  error_codes?: string[];
  error_routes_pattern?: string;
  expected_distinct_pages?: number;
};

export const siteDir = path.resolve(__dirname, "../..");
export const manifestPath = process.env.ROUTE_MANIFEST_PATH
  ? path.resolve(process.env.ROUTE_MANIFEST_PATH)
  : path.join(siteDir, "route-manifest.json");
export const buildDir = process.env.DOCS_SITE_BUILD_DIR
  ? path.resolve(process.env.DOCS_SITE_BUILD_DIR)
  : path.join(siteDir, "build");

export function loadManifest(): RouteManifest {
  return JSON.parse(fs.readFileSync(manifestPath, "utf-8"));
}

export function manifestRoutePaths(manifest = loadManifest()): string[] {
  const paths = manifest.routes.map((route) => route.path);
  for (const code of manifest.error_codes ?? []) {
    paths.push(`/errors/${code}`);
  }
  return paths;
}

export function koPath(route: string): string {
  return route === "/" ? "/ko/" : `/ko${route}`;
}

export function jaPath(route: string): string {
  return route === "/" ? "/ja/" : `/ja${route}`;
}

export function zhPath(route: string): string {
  return route === "/" ? "/zh-Hans/" : `/zh-Hans${route}`;
}

export function buildPageRoutes(buildRoot = buildDir): string[] {
  const routes: string[] = [];
  function walk(dir: string, prefix: string) {
    if (!fs.existsSync(dir)) return;
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        const nextPrefix = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
        walk(full, nextPrefix);
      } else if (entry.name === "index.html") {
        routes.push(prefix === "" ? "/" : `/${prefix}`);
      } else if (entry.name.endsWith(".html")) {
        const name = entry.name.replace(/\.html$/, "");
        const routePrefix = prefix === "" ? name : `${prefix}/${name}`;
        routes.push(`/${routePrefix}`);
      }
    }
  }
  walk(buildRoot, "");
  const skip = new Set([
    "/404", "/ko/404", "/ja/404", "/zh-Hans/404",
    "/markdown-page", "/ko/markdown-page", "/ja/markdown-page", "/zh-Hans/markdown-page",
    "/manifest", "/ko/manifest", "/ja/manifest", "/zh-Hans/manifest"
  ]);
  return routes.filter((r) => !skip.has(r.replace(/\/$/, "")));
}

