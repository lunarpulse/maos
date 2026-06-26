import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

// URL Contract (D6 — frozen day-one):
//   /manifest/   /cookbook/   /migrate/   /troubleshoot/
//   /deploy/     /errors/<ERR_NAME>      /abi/<version>/
//   /community/governance   /community/rfc-template
// Redirects configured below; static `serve` emits client-side redirects.
// Production hosting/CDN must bind these redirects as HTTP 301s for non-JS tools.

const config: Config = {
  title: "MAOS",
  tagline: "Multi-Agent Orchestration Substrate",
  favicon: "img/favicon.ico",

  url: "https://docs.maos.dev",
  baseUrl: "/",

  // Hard-fail on broken links/anchors (AC-1, NFR-Doc-1)
  onBrokenLinks: "throw",
  onBrokenAnchors: "throw",
  onDuplicateRoutes: "throw",

  markdown: {
    hooks: {
      onBrokenMarkdownLinks: "throw",
    },
  },

  i18n: {
    defaultLocale: "en",
    locales: ["en", "ko", "ja", "zh-Hans"],
    localeConfigs: {
      en: { label: "English", htmlLang: "en" },
      ko: { label: "한국어", htmlLang: "ko" },
      ja: { label: "日本語", htmlLang: "ja" },
      "zh-Hans": { label: "简体中文", htmlLang: "zh-Hans" },
    },
  },

  presets: [
    [
      "classic",
      {
        docs: {
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
          // Versioned docs for ABI reference archive (≥2 minor versions back)
          includeCurrentVersion: true,
          lastVersion: "current",
          versions: {
            current: { label: "Latest", path: "" },
          },
        },
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      "@docusaurus/plugin-content-docs",
      {
        id: "abi",
        path: "abi/v1",
        routeBasePath: "abi/v1",
        sidebarPath: "./sidebars-abi.ts",
      },
    ],
    [
      "@docusaurus/plugin-client-redirects",
      {
        redirects: [
          // D6 frozen URL contract — static `serve` proves client-side redirect target resolution.
          { from: "/schema", to: "/manifest/latest" },
          { from: "/errors", to: "/troubleshoot/" },
          { from: "/reference", to: "/abi/v1/" },
          { from: "/runbooks", to: "/deploy/" },
          { from: "/patterns", to: "/cookbook/" },
          { from: "/breaking", to: "/migrate/" },
          { from: "/abi", to: "/abi/v1/" },
          { from: "/abi/latest", to: "/abi/v1/" },
          { from: "/abi/cancellation", to: "/abi/v1/cancellation" },
          { from: "/abi/compliance", to: "/abi/v1/compliance" },
          { from: "/abi/constants", to: "/abi/v1/constants" },
          { from: "/abi/ctx", to: "/abi/v1/ctx" },
          { from: "/abi/deprecation", to: "/abi/v1/deprecation" },
          { from: "/abi/gateway", to: "/abi/v1/gateway" },
          { from: "/abi/identity", to: "/abi/v1/identity" },
          { from: "/abi/index", to: "/abi/v1/" },
          { from: "/abi/lifecycle", to: "/abi/v1/lifecycle" },
        ],
      },
    ],
  ],

  themeConfig: {
    navbar: {
      title: "MAOS",
      items: [
        { to: "/manifest/latest", label: "Manifest", position: "left" },
        { to: "/cookbook/", label: "Cookbook", position: "left" },
        { to: "/migrate/", label: "Migrate", position: "left" },
        { to: "/troubleshoot/", label: "Troubleshoot", position: "left" },
        { to: "/deploy/", label: "Deploy", position: "left" },
        { to: "/abi/v1/", label: "ABI Reference", position: "left" },
        {
          to: "/community/governance",
          label: "Community",
          position: "left",
        },
        {
          type: "localeDropdown",
          position: "right",
        },
        {
          href: "https://github.com/lunarpulse/maos",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Docs",
          items: [
            { label: "Write a Spirit", to: "/write-a-spirit" },
            { label: "Run MAOS", to: "/run-maos" },
            { label: "Understand MAOS", to: "/understand-maos" },
          ],
        },
        {
          title: "Reference",
          items: [
            { label: "Manifest Schema", to: "/manifest/latest" },
            { label: "ABI Reference", to: "/abi/v1/" },
            { label: "Error Catalog", to: "/troubleshoot/" },
          ],
        },
        {
          title: "Community",
          items: [
            { label: "Governance", to: "/community/governance" },
            { label: "Code of Conduct", to: "/community/code-of-conduct" },
            { label: "RFC Template", to: "/community/rfc-template" },
            { label: "GitHub", href: "https://github.com/lunarpulse/maos" },
          ],
        },
      ],
      copyright: `Copyright \u00a9 ${new Date().getFullYear()} MAOS Contributors. Apache-2.0 OR MIT.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ["toml", "bash", "rust"],
    },
    // WCAG AA: high contrast color mode available.
    // respectPrefersColorScheme is OFF so the toggle is a deterministic 2-state
    // light<->dark switch. With it ON, Docusaurus cycles system->light->dark and the
    // first click from "system" on a light OS is a visual no-op (reads as broken).
    colorMode: {
      defaultMode: "light",
      disableSwitch: false,
      respectPrefersColorScheme: false,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
