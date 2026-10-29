// @ts-check
import { themes as prismThemes } from "prism-react-renderer";

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Quantus",
  tagline: "A calculator for physical quantities, with LaTeX in and LaTeX out",
  favicon: "img/icon.svg",

  url: "https://iswearimmrtobi.github.io",
  // GitHub Pages serves the site under /quantus/; the dev server serves it at the root.
  baseUrl: process.env.NODE_ENV === "development" ? "/" : "/quantus/",
  organizationName: "iswearimmrtobi",
  projectName: "quantus",
  onBrokenLinks: "throw",

  // The existing documents are plain CommonMark with <, { and } in the text, so they must not be parsed as MDX.
  markdown: {
    format: "detect",
    hooks: { onBrokenMarkdownLinks: "warn" },
  },

  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          routeBasePath: "/",
          sidebarPath: "./sidebars.js",
          editUrl: "https://github.com/iswearimmrtobi/quantus/edit/master/docs/",
        },
        blog: false,
        theme: { customCss: "./src/css/custom.css" },
      }),
    ],
  ],

  themeConfig: {
    navbar: {
      title: "Quantus",
      logo: { alt: "Quantus", src: "img/icon.svg" },
      items: [
        { type: "docSidebar", sidebarId: "docs", position: "left", label: "Docs" },
        { href: "https://github.com/iswearimmrtobi/quantus", label: "GitHub", position: "right" },
      ],
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ["rust", "latex", "bash"],
    },
  },
};

export default config;
