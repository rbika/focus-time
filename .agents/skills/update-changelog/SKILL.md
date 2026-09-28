---
name: update-changelog
description: Updates CHANGELOG.md ## Unreleased from commits and diffs since the latest v* tag. Use when the user invokes /update-changelog or asks to fill Unreleased from git history.
disable-model-invocation: true
---

# Update Changelog

Fill `## Unreleased` in `CHANGELOG.md` from **committed** work since the latest `v*` tag. Edit the file. Do not commit, tag, or bump the version.

## Dialect

This file is not generic Keep a Changelog.

- Heading: `## Unreleased` (no brackets), above the latest `## v…`
- Categories, in this order only: `### New`, `### Improved`, `### Fixed`
- No Added / Changed / Removed / Deprecated / Security
- Removals and behavior changes go under **Improved**
- Voice: user-facing Mac app ship notes, same as existing bullets. Prefer `CONTEXT.md` terms (Entry, Interval, Stats — not session, log, Statistics-the-window).
- **New** examples: `- **Keyboard shortcuts** — See the full list in Settings → Shortcuts.` / `- New completion sounds.`
- **Improved** example: `- Settings is now organized into Tabs.`
- **Fixed** example: `- Midnight split logic from a running timer and from saving an entry.`

## Process

### 1. Collect

```bash
tag="$(git tag --list 'v*' --sort=-version:refname | head -1)"
git log --reverse --pretty=format:'%h %s' "$tag"..HEAD
git diff "$tag"..HEAD
```

If there is no `v*` tag, stop and say so.

Source of changes is that range only (committed). Uncommitted code is not a change. Read **working-tree** `CHANGELOG.md` as the merge base so uncommitted Unreleased bullets stay.

### 2. Evaluate every commit

A commit is **used** only if a Mac app user would expect it in a GitHub release body.

**Not used** (unless the diff is still user-visible): `chore`, merge, version-bump (`chore: update app version`), changelog-only, hook/CI/docs-only, internal seams, shared primitives, refactors.

Conventional-commit type is a hint, not a category. After the user-visible test:

- new capability → **New**
- existing surface better, removal, or behavior change → **Improved**
- bug → **Fixed**

Do not mint one “UI and UX improvements” bullet per refactor. Write that vague Improved bullet only when it is the only honest user-facing leftover and it matches existing notes.

Do not invent filler. Several commits may collapse into one bullet.

### 3. Write

- If `## Unreleased` is missing, insert it after the preamble and before the first `## v…`. If it exists elsewhere, move it there without dropping bullets.
- **Merge and dedupe.** Keep bullets already in Unreleased. Append new ones at the end of the matching category. Do not wipe and regenerate.
- Create a category heading only when it has bullets. Omit empty categories.
- Keep `### New` / `### Improved` / `### Fixed` in that order.
- If nothing is user-facing: leave `## Unreleased` with no categories or bullets, and say so in chat.

### 4. Stop and report

Do not `git add`, commit, tag, or bump `Cargo.toml`.

Reply with:

1. The `## Unreleased` section as written (or “empty, nothing user-facing”).
2. Every commit in `$tag..HEAD`, oldest first, marked **used** (which bullet) or **not used** (why).
