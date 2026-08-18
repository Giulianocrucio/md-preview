# Fork release checklist

This repository is derived from [vorojar/md-preview](https://github.com/vorojar/md-preview) under the MIT License. The current development branch adds a desktop workspace explorer and replaceable preview tabs. It is ready for local review, but it must not be presented as an independently maintained binary release until the ownership, update, signing, and store settings below have been replaced.

This checklist is practical project guidance, not legal advice.

## Source publication

- Keep `LICENSE`, including the original copyright and permission notice, in every source distribution.
- Describe the repository as a fork and link to the upstream project. Do not imply upstream endorsement.
- If code, artwork, fonts, or other assets are later copied from another project, record their source and license before committing them. The current unpublished feature delta adds no third-party dependency or asset.
- Publish through a GitHub fork when possible so GitHub preserves the upstream relationship. If using a standalone repository, keep the explicit attribution in README.md.
- Point the writable `origin` remote at the fork owner and keep the original repository as a read-only `upstream` remote. Verify with `git remote -v` before any push.

The MIT license permits use, modification, and publication when its copyright and permission notice are included. Branding, signing certificates, store listings, and service accounts are separate from the source-code license and must not be assumed to transfer with a fork.

## Independent binary release

Before building or tagging a release, replace and review every upstream-owned value found by:

```bash
rg -n "vorojar|com\.mdpreview|app\.mdpreview|github\.io|apps\.apple\.com" .
```

At minimum, review:

- GitHub repository, issue, release, website, and badge URLs in the READMEs, `docs/`, `src/main.rs`, and release scripts.
- The desktop update API and allowed download URLs. A forked build must never install an upstream binary as its own update.
- The macOS bundle identifier, Finder extension identifier, Sparkle feed URL, and a newly generated Sparkle signing key pair.
- Windows company/copyright metadata and release checksum flow.
- iOS and Android application identifiers, signing credentials, privacy/support pages, and store listings. Do not reuse an upstream App Store or Play Store identity without authorization.
- Product name, icon, screenshots, and other branding if the fork will be distributed as a separate product.
- Repository secrets used by CI, notarization, signing, or publishing. Never commit credentials or reuse another maintainer's credentials.

Run a release dry-run or use a temporary tag only after those values are owned by the fork maintainer. Inspect every generated archive and its update metadata before making a public release.

## Feature acceptance checks

Example input:

```text
md-preview path/to/docs/
```

Expected result:

1. The requested workspace replaces any previously restored workspace, starts without an open document, and shows the explorer expanded.
2. The explorer lists supported Markdown/text files while hidden and build directories remain filtered out.
3. A single file click reuses one italic preview tab; a double-click or edit pins it.
4. Restart restores pinned tabs and the last active pinned tab, but not the transient preview tab.
5. `Cmd/Ctrl+B` collapses and restores the explorer; narrow windows use an overlay and print output hides all explorer chrome.

Failure examples:

- An old or automatically selected document is active after launching a directory.
- Repeated single clicks create an unlimited number of tabs.
- Restart restores a transient preview or selects an unrelated pinned tab.
- A forked build checks, downloads, or installs releases from `vorojar/md-preview`.

## Suggested release presentation

### Workspace Explorer and Preview Tabs

This release turns MD Preview into a faster documentation workspace while keeping its local-first, lightweight design.

- Open a directory to get an expanded Markdown/text explorer with no document selected yet.
- Single-click files into one reusable preview tab, then double-click or edit to keep the document open.
- Preserve expanded folders while switching files, and restore pinned tabs with the correct active document after restart.
- Keep the explorer responsive in narrow windows and out of printed output.
- Protect existing autosave, missing-file, live-reload, and local-link behavior with regression tests.

State clearly whether the publication is source-only or includes signed binaries, identify the upstream project, link the MIT license, and list which platforms were actually built and tested.
