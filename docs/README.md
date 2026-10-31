# Documentation

Plain Markdown, tracked in the repository. There is no site generator config and
no deployed site (the generated site was removed in 0.2.15) - read these files in
a checkout, on GitHub, or in any Markdown viewer.

New here? Start with **[intro.md](intro.md)**.

## Where to look

| I want to...                                                   | Read                                                                                                         |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Install and make a first call                                  | [intro.md](intro.md), then [api/reference.md](api/reference.md)                                              |
| Use the Python client (async and sync)                         | [api/python.md](api/python.md)                                                                               |
| Copy a runnable example                                        | [examples/](examples/index.md)                                                                               |
| Learn trading patterns (Martingale, trend following, risk)     | [guides/trading.md](guides/trading.md)                                                                       |
| Build a full Python bot                                        | [guides/python-pystrategy-trading-bot.md](guides/python-pystrategy-trading-bot.md)                           |
| Send raw frames and filter messages                            | [guides/raw-handler.md](guides/raw-handler.md)                                                               |
| Understand why a candle call returned fewer candles than asked | [guides/candle-history-limits.md](guides/candle-history-limits.md)                                           |
| Check whether an asset is trading and what timeframes exist    | [guides/assets-timeframes.md](guides/assets-timeframes.md)                                                   |
| Understand the internals                                       | [architecture/structure.md](architecture/structure.md), [architecture/dataflow.md](architecture/dataflow.md) |
| Work on the docs themselves                                    | [project/deployment.md](project/deployment.md)                                                               |

## Layout

```
docs/
├── README.md          # this file - the index
├── intro.md           # what the project is, quick links
├── api/               # reference.md (multi-language), python.md
├── guides/            # trading, raw handler, candle limits, assets, bot walkthrough
├── architecture/      # structure.md, dataflow.md, raw-module.md
├── examples/          # per-language runnable examples
├── project/           # deployment, the 0.2.6 breaking changes, simplification plan
├── ecosystem.md       # ChipaEditor and ChipaX
└── tutorials/         # getting an SSID, screenshots, userscript
```

`data/` holds captured fixtures (asset lists, a candle CSV) used when explaining
broker limits, not prose.

## Conventions

- Code examples must match the real signatures. When a method changes, update
  every page that shows it - `docs/api/` first, then `examples/`, then the
  per-crate READMEs.
- Broker-measured limits (how many symbols stream at once, how fast history
  requests may be issued) name their source and date where they are defined, and
  the guides explain the consequence. Do not restate a limit as a fact without
  that provenance - an earlier revision of `guides/candle-history-limits.md`
  documented the wrong meaning for `offset` because it was never checked against
  the implementation.
- Frontmatter (`slug`, `sidebar_position`) is left over from the removed site.
  Keep it consistent when editing an existing page; it is not required for new
  ones.
