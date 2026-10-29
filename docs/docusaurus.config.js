// @ts-check
import { themes as prismThemes } from "prism-react-renderer";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Quantus",
  tagline: "A calculator for physical quantities, with LaTeX in and LaTeX out",
  favicon: "img/icon.svg",

  // GitHub Pages serves the site on its own domain, so the landing page is at the root and the docs under /docs/.
  url: "https://quantus.tobifrosch.dev",
  baseUrl: "/",
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
          sidebarPath: "./sidebars.js",
          editUrl: "https://github.com/iswearimmrtobi/quantus/edit/master/docs/",
          remarkPlugins: [remarkMath],
          rehypePlugins: [rehypeKatex],
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
