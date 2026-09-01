// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  // By default, Docusaurus generates a sidebar from the docs folder structure
  tutorialSidebar: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Getting Started',
    },
    {
      type: 'doc',
      id: 'OVERVIEW',
      label: 'Overview',
    },
    {
      type: 'doc',
      id: 'ecosystem',
      label: '✨ Chipa Ecosystem',
    },
    {
      type: 'category',
      label: 'API Reference',
      items: [
        'api/reference',
        'api/python',
      ],
    },
    {
      type: 'category',
      label: 'Examples',
      items: [
        {
          type: 'category',
          label: 'Python',
          items: [
            'examples/python/async/index',
            'examples/python/sync/index',
          ],
        },
        'examples/rust/index',
        'examples/javascript/index',
        'examples/swift/index',
        'examples/kotlin/index',
        'examples/go/index',
        'examples/ruby/index',
        'examples/csharp/index',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      items: [
        'guides/trading',
        'guides/raw-handler',
        'guides/assets-timeframes',
        'guides/candle-history-limits',
        'guides/python-pystrategy-trading-bot',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      items: [
        'architecture/structure',
        'architecture/dataflow',
        'architecture/raw-module',
      ],
    },
    {
      type: 'category',
      label: 'Tutorials',
      items: [
        'tutorials/index',
        {
          type: 'category',
          label: 'Scripts',
          items: [
            'tutorials/scripts/index',
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Project Info',
      items: [
        {
          type: 'link',
          label: 'Contributing',
          href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/blob/master/CONTRIBUTING.md',
        },
        {
          type: 'link',
          label: 'Code of Conduct',
          href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/blob/master/CODE_OF_CONDUCT.md',
        },
        {
          type: 'link',
          label: 'Security',
          href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/blob/master/SECURITY.md',
        },
        {
          type: 'link',
          label: 'License',
          href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/blob/master/LICENSE',
        },
        {
          type: 'link',
          label: 'Acknowledgments',
          href: 'https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/blob/master/ACKNOWLEDGMENTS.md',
        },
        'project/deployment',
        'project/breaking-changes-0.2.6',
        'project/raw-handler-summary',
      ],
    },
    {
      type: 'category',
      label: 'Chipa Ecosystem',
      items: [
        'ecosystem',
        {
          type: 'link',
          label: '✨ ChipaEditor — Strategy Builder',
          href: 'https://chipaeditor.com/?utm_source=docs&utm_medium=sidebar&utm_campaign=BinaryOptionsToolsV2',
        },
        {
          type: 'link',
          label: '📈 ChipaX — Crypto Exchange',
          href: 'https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS',
        },
      ],
    },
  ],
};

module.exports = sidebars;