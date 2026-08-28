import React from 'react';
import ComponentCreator from '@docusaurus/ComponentCreator';

export default [
  {
    path: '/BinaryOptionsTools-v2/search',
    component: ComponentCreator('/BinaryOptionsTools-v2/search', '31c'),
    exact: true
  },
  {
    path: '/BinaryOptionsTools-v2/',
    component: ComponentCreator('/BinaryOptionsTools-v2/', '885'),
    routes: [
      {
        path: '/BinaryOptionsTools-v2/',
        component: ComponentCreator('/BinaryOptionsTools-v2/', '75c'),
        routes: [
          {
            path: '/BinaryOptionsTools-v2/',
            component: ComponentCreator('/BinaryOptionsTools-v2/', '1b9'),
            routes: [
              {
                path: '/BinaryOptionsTools-v2/api/python',
                component: ComponentCreator('/BinaryOptionsTools-v2/api/python', 'ef8'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/api/reference',
                component: ComponentCreator('/BinaryOptionsTools-v2/api/reference', 'd91'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/architecture/dataflow',
                component: ComponentCreator('/BinaryOptionsTools-v2/architecture/dataflow', 'bff'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/architecture/raw-module',
                component: ComponentCreator('/BinaryOptionsTools-v2/architecture/raw-module', '1e7'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/architecture/structure',
                component: ComponentCreator('/BinaryOptionsTools-v2/architecture/structure', '5fc'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/ecosystem',
                component: ComponentCreator('/BinaryOptionsTools-v2/ecosystem', '195'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples', 'a9e'),
                exact: true
              },
              {
                path: '/BinaryOptionsTools-v2/examples/csharp',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/csharp', 'c79'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/go',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/go', '2b0'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/javascript',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/javascript', 'e8f'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/kotlin',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/kotlin', 'ea0'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/python/async',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/python/async', 'c5a'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/python/sync',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/python/sync', '48c'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/ruby',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/ruby', 'a40'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/rust',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/rust', '32e'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/examples/swift',
                component: ComponentCreator('/BinaryOptionsTools-v2/examples/swift', '2bc'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/guides/assets-timeframes',
                component: ComponentCreator('/BinaryOptionsTools-v2/guides/assets-timeframes', '983'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/guides/python-pystrategy-trading-bot',
                component: ComponentCreator('/BinaryOptionsTools-v2/guides/python-pystrategy-trading-bot', '30d'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/guides/raw-handler',
                component: ComponentCreator('/BinaryOptionsTools-v2/guides/raw-handler', 'f00'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/guides/trading',
                component: ComponentCreator('/BinaryOptionsTools-v2/guides/trading', '86e'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/intro',
                component: ComponentCreator('/BinaryOptionsTools-v2/intro', 'c7f'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/macro_proposals',
                component: ComponentCreator('/BinaryOptionsTools-v2/macro_proposals', '3e0'),
                exact: true
              },
              {
                path: '/BinaryOptionsTools-v2/overview',
                component: ComponentCreator('/BinaryOptionsTools-v2/overview', '57e'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/project/breaking-changes-0.2.6',
                component: ComponentCreator('/BinaryOptionsTools-v2/project/breaking-changes-0.2.6', '49a'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/project/deployment',
                component: ComponentCreator('/BinaryOptionsTools-v2/project/deployment', '7eb'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/project/raw-handler-summary',
                component: ComponentCreator('/BinaryOptionsTools-v2/project/raw-handler-summary', 'af9'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/tutorials',
                component: ComponentCreator('/BinaryOptionsTools-v2/tutorials', '1c9'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/tutorials/scripts',
                component: ComponentCreator('/BinaryOptionsTools-v2/tutorials/scripts', '0fc'),
                exact: true,
                sidebar: "tutorialSidebar"
              },
              {
                path: '/BinaryOptionsTools-v2/',
                component: ComponentCreator('/BinaryOptionsTools-v2/', '0cb'),
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
