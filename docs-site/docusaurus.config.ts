import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

// URL Contract (D6 — frozen day-one):
//   /manifest/   /cookbook/   /migrate/   /troubleshoot/
//   /deploy/     /errors/<ERR_NAME>      /abi/<version>/
//   /community/governance   /community/rfc-template
// Redirects configured below; reorg degrades to 301, not 404.

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
    locales: ["en", "ko"],
    localeConfigs: {
      en: { label: "English", htmlLang: "en" },
      ko: { label: "한국어", htmlLang: "ko" },
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
      "@docusaurus/plugin-client-redirects",
      {
        redirects: [
          // D6 frozen URL contract — day-one redirects so reorg → 301, not 404
          { from: "/schema", to: "/manifest/latest" },
          { from: "/errors", to: "/troubleshoot/" },
          { from: "/reference", to: "/abi/latest" },
          { from: "/runbooks", to: "/deploy/" },
          { from: "/patterns", to: "/cookbook/" },
          { from: "/breaking", to: "/migrate/" },
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
        { to: "/abi/latest", label: "ABI Reference", position: "left" },
        {
          to: "/community/governance",
          label: "Community",
          position: "left",
        },
        {
          type: "docsVersionDropdown",
          position: "right",
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
            { label: "ABI Reference", to: "/abi/latest" },
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
    // WCAG AA: high contrast color mode available
    colorMode: {
      defaultMode: "light",
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
