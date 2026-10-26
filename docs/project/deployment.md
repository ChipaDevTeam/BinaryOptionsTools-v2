---
sidebar_position: 1
slug: /project/deployment
---

# Documentation Deployment

The documentation in `docs/` is a set of Markdown files with Docusaurus-style
frontmatter (`sidebar_position`, `slug`). **No site generator configuration is
tracked in this repository**: `docusaurus.config.js`, `sidebars.js` and the
`docs-site/` package were deleted in 0.2.15, and the repository has no root Node
manifest. The Markdown is the deliverable; building a site from it needs the
generator to be set up again.

## What is in the repository

```
docs/
├── intro.md              # Homepage
├── OVERVIEW.md           # Documentation overview
├── ecosystem.md          # Chipa ecosystem
├── api/                  # API reference
├── guides/               # Trading guides
├── architecture/         # Architecture docs
├── examples/             # Code examples
├── tutorials/            # Step-by-step tutorials
├── project/              # Project info
├── data/                 # Asset lists and CSV fixtures
└── assets/               # Images
```

## Reading the docs without a site

Every page is plain Markdown and reads correctly on its own — for example
[`docs/OVERVIEW.md`](../OVERVIEW.md) or [`docs/api/reference.md`](../api/reference.md).
Relative links between pages resolve in a checkout (GitHub, GitLab, or a local
editor preview).

## Re-creating a site (optional)

If you want a hosted site again, the frontmatter is already Docusaurus-shaped:

1. Create a dedicated directory with its own `package.json` (for example
   `docs-site/`), and keep it out of `docs/` so the Markdown stays readable.
2. Add `@docusaurus/core` and `@docusaurus/preset-classic`, point
   `docs: { path: '../docs' }` at this directory, and add the standard
   `docusaurus.config.js`.
3. Serve locally with the usual `npm run start`, and build with `npm run build`.
4. Deploy the built output with whatever CI you choose. Note that this
   repository's `.gitlab-ci.yml` only builds wheels, and
   `.github/workflows/CI.yml` only builds wheels, the Node addon and the
   release — neither has a docs job.

## Troubleshooting

### A page link does not resolve

Links are relative to the file that contains them. Check the target exists in
this tree before changing the link; several historical links pointed at pages
that were renamed (for example `docs/macro_examples.rs`, now
`crates/macros/docs/macro_examples.rs`).

### A `npm run …` command from an older doc fails

There is no root `package.json`, so npm commands only work inside
`nodejs/` (the Node addon) or inside a site directory you create yourself.

## Support

For documentation issues:

1. Read the page in a Markdown preview to confirm the content itself is fine.
2. Check that the paths you reference exist in `git ls-files`.
3. Open an issue on the repository with the file and line.
