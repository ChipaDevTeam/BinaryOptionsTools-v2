import { themes as prismThemes } from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'BinaryOptionsTools V2',
  tagline: 'The most advanced binary options trading library for Python, JavaScript, and Rust.',
  favicon: 'img/favicon.svg',

  // Set the production url of your site here
  url: 'https://chipatrade.gitlab.io',
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitLab Pages deployment, it is often '/<projectName>/'
  baseUrl: '/chipadevorg/BinaryOptionsTools-v2/',

  // GitLab pages deployment config.
  organizationName: 'chipadevorg',
  projectName: 'BinaryOptionsTools-v2',
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          path: '../docs',
          routeBasePath: '/',
          editUrl: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/tree/master/docs/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      // Replace with your project's social card
      image: 'img/binary-options-social-card.jpg',
      navbar: {
        title: 'BinaryOptionsTools V2',
        logo: {
          alt: 'BinaryOptionsTools Logo',
          src: 'img/logo.svg',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'tutorialSidebar',
            position: 'left',
            label: 'Documentation',
          },
          {
            to: '/api/reference',
            label: 'API Reference',
            position: 'left',
          },
          {
            to: '/examples',
            label: 'Examples',
            position: 'left',
          },
          {
            href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2',
            label: 'GitLab',
            position: 'right',
          },
          {
            href: 'https://discord.gg/p7YyFqSmAz',
            label: 'Discord',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Documentation',
            items: [
              {
                label: 'Getting Started',
                to: '/',
              },
              {
                label: 'API Reference',
                to: '/api/reference',
              },
              {
                label: 'Examples',
                to: '/examples',
              },
            ],
          },
          {
            title: 'Community',
            items: [
              {
                label: 'Discord',
                href: 'https://discord.gg/p7YyFqSmAz',
              },
              {
                label: 'GitHub Issues',
                label: 'GitLab Issues',
                href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/issues',
              },
            ],
          },
          {
            title: 'More',
            items: [
              {
                label: 'GitHub',
                label: 'GitLab',
                href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2',
              },
              {
                label: 'PyPI',
                href: 'https://pypi.org/project/BinaryOptionsToolsV2/',
              },
              {
                label: 'crates.io',
                href: 'https://crates.io/crates/binary_options_tools',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} ChipaDevTeam. Built with Docusaurus.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['python', 'rust', 'kotlin', 'swift', 'go', 'ruby', 'csharp', 'javascript', 'typescript'],
      },
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
      algolia: {
        // The application ID provided by Algolia
        appId: 'YOUR_APP_ID',
        // Public API key: it is safe to commit it
        apiKey: 'YOUR_API_KEY',
        indexName: 'binaryoptionstools',
        contextualSearch: true,
        externalUrlRegex: 'external\\.com',
      },
    }),
};

export default config;