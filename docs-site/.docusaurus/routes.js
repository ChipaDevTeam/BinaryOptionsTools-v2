import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/chipadevorg/BinaryOptionsTools-v2/search',
    component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/search', 'e89'),
    exact: true
  },
  {
    path: '/chipadevorg/BinaryOptionsTools-v2/',
    component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/', '07d'),
    routes: [
      {
        path: '/chipadevorg/BinaryOptionsTools-v2/',
        component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/', 'c71'),
        routes: [
          {
            path: '/chipadevorg/BinaryOptionsTools-v2/',
            component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/', 'fda'),
            routes: [
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/api/python',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/api/python', '740'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/api/reference',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/api/reference', '37d'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/architecture/dataflow',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/architecture/dataflow', '0cb'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/architecture/raw-module',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/architecture/raw-module', '065'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/architecture/structure',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/architecture/structure', '634'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples', '4e3'),
                exact: true
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/csharp',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/csharp', '608'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/go',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/go', '22b'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/javascript',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/javascript', '810'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/kotlin',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/kotlin', '1d5'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/python/async',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/python/async', '3e4'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/python/sync',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/python/sync', 'fb0'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/ruby',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/ruby', '0ca'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/rust',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/rust', '297'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/examples/swift',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/examples/swift', 'ef9'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/guides/assets-timeframes',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/guides/assets-timeframes', '842'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/guides/python-pystrategy-trading-bot',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/guides/python-pystrategy-trading-bot', 'bb3'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/guides/raw-handler',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/guides/raw-handler', '308'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/guides/trading',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/guides/trading', '118'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/intro',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/intro', '5c1'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/macro_proposals',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/macro_proposals', 'f85'),
                exact: true
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/overview',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/overview', '1f2'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/project/breaking-changes-0.2.6',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/project/breaking-changes-0.2.6', '0ec'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/project/deployment',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/project/deployment', '057'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/project/raw-handler-summary',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/project/raw-handler-summary', 'ee5'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/tutorials',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/tutorials', '5c7'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/tutorials/scripts',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/tutorials/scripts', '031'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/chipadevorg/BinaryOptionsTools-v2/',
                component: ComponentCreator('/chipadevorg/BinaryOptionsTools-v2/', '5d7'),
                exact: true
              }
            ]
          }
        ]
      }
    ]
  },
  {
    path: '*',
    component: ComponentCreator('*'),
  },
];
