# Analysis of FrankenTUI Spec Doc Diffs

This is a *living*, structured audit of how the FrankenTUI spec corpus evolved over time, derived from **git commit diffs** and **manual, post-hoc categorization**.

**Scope (current):** `docs/spec/**` + `docs/specs/**` in `/data/projects/frankentui`.

## Buckets (Multi-Label)

A single logical change can belong to multiple buckets. For visualization, we can later choose between:
- **Soft assignment (recommended for charts):** weights sum to 1 per change-group
- **Multi-label counts:** a change contributes fully to each selected bucket (over-counting totals)

| # | Bucket | FrankenTUI-Specific Interpretation |
|---:|---|---|
| 1 | Logic/math/reasoning mistake fixes | Correct incorrect claims, equations, invariants, or reasoning inside the spec |
| 2 | Inaccurate statements about FrankenTUI codebase | Fix spec claims that disagree with actual ftui code / behavior |
| 3 | Inaccurate statements about external terminal ecosystems | Fix claims about terminals/muxes/ANSI, or external libs/tools (tmux, Ghostty, etc.) |
| 4 | Conceptual / architectural mistake fixes | Fix wrong abstractions, layering, API boundaries, invariants |
| 5 | Ministerial / scrivening fixes | Renumbering, references, wording, formatting, link fixes |
| 6 | Add background / context | Add explanatory material so spec is self-contained |
| 7 | Standard engineering improvements | Perf / concurrency / caching / robustness improvements described in spec (non-alien) |
| 8 | Alien-artifact improvements | Add advanced math / formal guarantees (BOCPD, VOI, conformal, e-processes, etc.) |
| 9 | Clarification / elaboration (non-substantive) | Expand explanations without materially changing design or fixing an error |
| 10 | Other | Everything else |

## Metrics

We track multiple notions of "change magnitude":
- `+lines` / `-lines`: git numstat additions/deletions for spec paths
- `edit_distance`: Levenshtein distance between pre/post snapshots (per-file and/or aggregated corpus)
- `tokens`: word-like tokens (deterministic segmentation; not LLM tokens)

## Commit Index

| # | Commit | Date | Subject | Spec Δ (+/-) | Review |
|---:|---|---|---|---:|---|
| 1 | `aadc5679` | 2026-01-31T23:23:49-05:00 | feat: initialize Rust workspace with ftui crate structure | 104/0 | TODO |
| 2 | `67853b03` | 2026-02-01T16:21:22-05:00 | test: add ~55 tests for pty, text, style, logging modules | 212/0 | TODO |
| 3 | `b60a6e15` | 2026-02-02T19:41:53-05:00 | chore: sync multi-agent changes | 1010/1 | TODO |
| 4 | `1b4cd96c` | 2026-02-02T20:01:48-05:00 | docs(spec): add performance HUD and resize scheduler specifications | 504/0 | TODO |
| 5 | `36b0fce0` | 2026-02-02T20:04:12-05:00 | docs(spec): expand resize scheduler with implementation details | 67/0 | TODO |
| 6 | `da0ba795` | 2026-02-02T20:06:14-05:00 | refactor(runtime): minor input_macro adjustment and spec expansion | 80/25 | TODO |
| 7 | `0b030048` | 2026-02-02T20:17:48-05:00 | feat(widgets): add toast and notification queue widgets (bd-3tmk) | 394/0 | TODO |
| 8 | `2032fa02` | 2026-02-02T21:26:53-05:00 | docs(spec): add telemetry architecture specification (bd-1z02.2) | 168/0 | TODO |
| 9 | `a1cef754` | 2026-02-02T23:05:43-05:00 | docs(spec): add command palette and keybinding specs, expand E2E scripts | 399/0 | TODO |
| 10 | `5d813fe7` | 2026-02-02T23:44:43-05:00 | feat(text-effects): add gradient LUT optimization + multi-agent sync (bd-vzjn) | 309/0 | TODO |
| 11 | `8c884f3c` | 2026-02-03T09:27:58-05:00 | docs: comprehensive documentation updates and new specs | 461/0 | TODO |
| 12 | `0a527c82` | 2026-02-03T09:40:27-05:00 | docs: update fixes summary, review report, and telemetry specs | 22/1 | TODO |
| 13 | `ff65c775` | 2026-02-03T11:26:44-05:00 | feat(widgets): add comprehensive UI inspector widget | 55/0 | TODO |
| 14 | `9e3276b9` | 2026-02-03T11:27:31-05:00 | docs: update fixes summary, review report, and perf hud spec | 4/2 | TODO |
| 15 | `21bceab3` | 2026-02-03T13:40:18-05:00 | chore: sync workspace updates | 119/0 | TODO |
| 16 | `8fb8919e` | 2026-02-03T14:40:43-05:00 | bd-1rz0.28 VOI sampling policy | 343/0 | TODO |
| 17 | `66118060` | 2026-02-03T18:22:12-05:00 | docs: update state machine and telemetry event specifications | 245/0 | TODO |
| 18 | `4fcf96a4` | 2026-02-03T21:37:12-05:00 | docs: update BOCPD evidence fields and add unified evidence sink docs | 151/76 | TODO |
| 19 | `63ad3c80` | 2026-02-04T00:23:29-05:00 | docs: update documentation, session notes, and specifications | 101/1 | TODO |
| 20 | `f59fb5f8` | 2026-02-04T14:13:07-05:00 | bd-1e3w: demo nav hint + vfx visibility | 182/0 | TODO |
| 21 | `f42923b7` | 2026-02-04T19:20:04-05:00 | Sync workspace updates | 140/0 | TODO |
| 22 | `a38de75c` | 2026-02-04T21:47:12-05:00 | test(e2e): expand E2E test coverage with new scripts and validation | 108/0 | TODO |
| 23 | `0ccd68e0` | 2026-02-04T23:43:43-05:00 | refactor(style): improve table theme validation and add serde strictness | 49/12 | Reviewed |
| 24 | `d2c0e330` | 2026-02-05T00:44:30-05:00 | bd-2oovu: mermaid complexity guards | 17/4 | Reviewed |
| 25 | `5a122d89` | 2026-02-05T00:50:06-05:00 | chore: update docs, E2E scripts, beads, and session tracking | 48/0 | Reviewed |
| 26 | `19db685e` | 2026-02-05T01:31:47-05:00 | bd-yhp9: finalize upgrades and mermaid style order | 38/5 | Reviewed |
| 27 | `5a28cbaa` | 2026-02-05T03:26:18-05:00 | Expand mermaid terminal renderer with layout engine, subgraphs, and config system | 151/18 | Reviewed |
| 28 | `8c2762d2` | 2026-02-05T09:48:42-05:00 | Fix 6 bugs found in deep review of mermaid pipeline | 210/0 | Reviewed |
| 29 | `51e55477` | 2026-02-05T09:58:00-05:00 | Mermaid updates: samples + ER render | 44/0 | Reviewed |
| 30 | `6e034e31` | 2026-02-05T13:06:52-05:00 | enhance(mermaid): improve diagram rendering and layout algorithms | 32/0 | Reviewed |
| 31 | `7f463211` | 2026-02-06T17:02:06-05:00 | bd-hudcn.1.8.3: PTY JSONL parsing + journey ASCII snapshots | 8/4 | Reviewed |

---

<details>
<summary><strong>1. aadc5679</strong> — feat: initialize Rust workspace with ftui crate structure (2026-01-31T23:23:49-05:00)</summary>

**Metadata**
- Commit: `aadc567948ed52075e1e73af4d21f1971f76c86c`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-01-31T23:23:49-05:00
- Scope Δ (spec paths only): `+104 / -0`

**Files (spec paths only)**
- `docs/spec/state-machines.md` (`+104` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>2. 67853b03</strong> — test: add ~55 tests for pty, text, style, logging modules (2026-02-01T16:21:22-05:00)</summary>

**Metadata**
- Commit: `67853b03020d796814892a9ecca84af54f928531`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-01T16:21:22-05:00
- Scope Δ (spec paths only): `+212 / -0`

**Files (spec paths only)**
- `docs/spec/cache-and-layout.md` (`+212` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>3. b60a6e15</strong> — chore: sync multi-agent changes (2026-02-02T19:41:53-05:00)</summary>

**Metadata**
- Commit: `b60a6e15519bd05572e7fbf5e041f88c0674d2c1`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T19:41:53-05:00
- Scope Δ (spec paths only): `+1010 / -1`

**Files (spec paths only)**
- `docs/spec/log-search.md` (`+143` / `-0`)
- `docs/spec/macro-recorder.md` (`+161` / `-0`)
- `docs/spec/semantic-events.md` (`+195` / `-0`)
- `docs/spec/state-machines.md` (`+127` / `-1`)
- `docs/specs/ui-inspector.md` (`+384` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>4. 1b4cd96c</strong> — docs(spec): add performance HUD and resize scheduler specifications (2026-02-02T20:01:48-05:00)</summary>

**Metadata**
- Commit: `1b4cd96cbf9da88baec833f41c67551f0ad39b20`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T20:01:48-05:00
- Scope Δ (spec paths only): `+504 / -0`

**Files (spec paths only)**
- `docs/spec/performance-hud.md` (`+172` / `-0`)
- `docs/spec/resize-scheduler.md` (`+332` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>5. 36b0fce0</strong> — docs(spec): expand resize scheduler with implementation details (2026-02-02T20:04:12-05:00)</summary>

**Metadata**
- Commit: `36b0fce0becf0bcd72c01b2dc2e8eacc8b14311b`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T20:04:12-05:00
- Scope Δ (spec paths only): `+67 / -0`

**Files (spec paths only)**
- `docs/spec/resize-scheduler.md` (`+67` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>6. da0ba795</strong> — refactor(runtime): minor input_macro adjustment and spec expansion (2026-02-02T20:06:14-05:00)</summary>

**Metadata**
- Commit: `da0ba7953761028bc691086e8385fe8b74b373cc`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T20:06:14-05:00
- Scope Δ (spec paths only): `+80 / -25`

**Files (spec paths only)**
- `docs/spec/resize-scheduler.md` (`+80` / `-25`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>7. 0b030048</strong> — feat(widgets): add toast and notification queue widgets (bd-3tmk) (2026-02-02T20:17:48-05:00)</summary>

**Metadata**
- Commit: `0b030048e9b1470f9bc4469c34befbaf6a58ca3a`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T20:17:48-05:00
- Scope Δ (spec paths only): `+394 / -0`

**Files (spec paths only)**
- `docs/spec/command-palette.md` (`+394` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>8. 2032fa02</strong> — docs(spec): add telemetry architecture specification (bd-1z02.2) (2026-02-02T21:26:53-05:00)</summary>

**Metadata**
- Commit: `2032fa02946cecf0c4f26975be9fe2edc9f7bb7c`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T21:26:53-05:00
- Scope Δ (spec paths only): `+168 / -0`

**Files (spec paths only)**
- `docs/spec/telemetry.md` (`+168` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>9. a1cef754</strong> — docs(spec): add command palette and keybinding specs, expand E2E scripts (2026-02-02T23:05:43-05:00)</summary>

**Metadata**
- Commit: `a1cef754be9528486ec1fde7dd6b8b257c74820c`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T23:05:43-05:00
- Scope Δ (spec paths only): `+399 / -0`

**Files (spec paths only)**
- `docs/spec/command-palette.md` (`+29` / `-0`)
- `docs/spec/keybinding-policy.md` (`+370` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>10. 5d813fe7</strong> — feat(text-effects): add gradient LUT optimization + multi-agent sync (bd-vzjn) (2026-02-02T23:44:43-05:00)</summary>

**Metadata**
- Commit: `5d813fe7bdb8a0f8b6cc0a43b2757df6b847f509`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-02T23:44:43-05:00
- Scope Δ (spec paths only): `+309 / -0`

**Files (spec paths only)**
- `docs/spec/telemetry-events.md` (`+309` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>11. 8c884f3c</strong> — docs: comprehensive documentation updates and new specs (2026-02-03T09:27:58-05:00)</summary>

**Metadata**
- Commit: `8c884f3cd7d144db14dd2a3a95aac592b540cb99`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T09:27:58-05:00
- Scope Δ (spec paths only): `+461 / -0`

**Files (spec paths only)**
- `docs/spec/embedded-core.md` (`+84` / `-0`)
- `docs/spec/ffi-crate-layout.md` (`+91` / `-0`)
- `docs/spec/ffi-placeholder-policy.md` (`+39` / `-0`)
- `docs/spec/sdk-test-strategy.md` (`+68` / `-0`)
- `docs/spec/telemetry-perf.md` (`+178` / `-0`)
- `docs/spec/telemetry.md` (`+1` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>12. 0a527c82</strong> — docs: update fixes summary, review report, and telemetry specs (2026-02-03T09:40:27-05:00)</summary>

**Metadata**
- Commit: `0a527c8202be6702673382d0794e5c8c5b79fff1`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T09:40:27-05:00
- Scope Δ (spec paths only): `+22 / -1`

**Files (spec paths only)**
- `docs/spec/telemetry-events.md` (`+3` / `-1`)
- `docs/spec/telemetry-perf.md` (`+19` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>13. ff65c775</strong> — feat(widgets): add comprehensive UI inspector widget (2026-02-03T11:26:44-05:00)</summary>

**Metadata**
- Commit: `ff65c775b678d9f931e0b48c07433bfbc519daae`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T11:26:44-05:00
- Scope Δ (spec paths only): `+55 / -0`

**Files (spec paths only)**
- `docs/specs/ui-inspector.md` (`+55` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>14. 9e3276b9</strong> — docs: update fixes summary, review report, and perf hud spec (2026-02-03T11:27:31-05:00)</summary>

**Metadata**
- Commit: `9e3276b9095f1f4301cfbf21a874bc58c5b34a75`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T11:27:31-05:00
- Scope Δ (spec paths only): `+4 / -2`

**Files (spec paths only)**
- `docs/spec/performance-hud.md` (`+4` / `-2`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>15. 21bceab3</strong> — chore: sync workspace updates (2026-02-03T13:40:18-05:00)</summary>

**Metadata**
- Commit: `21bceab335149adbda6f5d86fb40ca1c04139de7`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T13:40:18-05:00
- Scope Δ (spec paths only): `+119 / -0`

**Files (spec paths only)**
- `docs/specs/command-palette.md` (`+119` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>16. 8fb8919e</strong> — bd-1rz0.28 VOI sampling policy (2026-02-03T14:40:43-05:00)</summary>

**Metadata**
- Commit: `8fb8919e3a86e260c4563f30c5dec065c07a6d77`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T14:40:43-05:00
- Scope Δ (spec paths only): `+343 / -0`

**Files (spec paths only)**
- `docs/spec/resize-migration.md` (`+290` / `-0`)
- `docs/spec/resize-scheduler.md` (`+53` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>17. 66118060</strong> — docs: update state machine and telemetry event specifications (2026-02-03T18:22:12-05:00)</summary>

**Metadata**
- Commit: `6611806097af4d9af2e241bd4c427385e9a07e89`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T18:22:12-05:00
- Scope Δ (spec paths only): `+245 / -0`

**Files (spec paths only)**
- `docs/spec/state-machines.md` (`+166` / `-0`)
- `docs/spec/telemetry-events.md` (`+79` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>18. 4fcf96a4</strong> — docs: update BOCPD evidence fields and add unified evidence sink docs (2026-02-03T21:37:12-05:00)</summary>

**Metadata**
- Commit: `4fcf96a4821f10e50d80d9db0b5e385673974abf`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-03T21:37:12-05:00
- Scope Δ (spec paths only): `+151 / -76`

**Files (spec paths only)**
- `docs/spec/resize-scheduler.md` (`+40` / `-19`)
- `docs/spec/state-machines.md` (`+28` / `-0`)
- `docs/spec/telemetry-events.md` (`+83` / `-57`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>19. 63ad3c80</strong> — docs: update documentation, session notes, and specifications (2026-02-04T00:23:29-05:00)</summary>

**Metadata**
- Commit: `63ad3c809f0feb6d1e112707159ed998cf2b58cb`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-04T00:23:29-05:00
- Scope Δ (spec paths only): `+101 / -1`

**Files (spec paths only)**
- `docs/spec/state-machines.md` (`+86` / `-0`)
- `docs/spec/telemetry-events.md` (`+15` / `-1`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>20. f59fb5f8</strong> — bd-1e3w: demo nav hint + vfx visibility (2026-02-04T14:13:07-05:00)</summary>

**Metadata**
- Commit: `f59fb5f80cf90a86ea427f83b38dc13ae1fd70ee`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-04T14:13:07-05:00
- Scope Δ (spec paths only): `+182 / -0`

**Files (spec paths only)**
- `docs/specs/table-theme.md` (`+182` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>21. f42923b7</strong> — Sync workspace updates (2026-02-04T19:20:04-05:00)</summary>

**Metadata**
- Commit: `f42923b7b41a1b62ced8d611f9a288eb3253a84e`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-04T19:20:04-05:00
- Scope Δ (spec paths only): `+140 / -0`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+140` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>22. a38de75c</strong> — test(e2e): expand E2E test coverage with new scripts and validation (2026-02-04T21:47:12-05:00)</summary>

**Metadata**
- Commit: `a38de75cba41c7e9fc7118dee6261014d58c563f`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-04T21:47:12-05:00
- Scope Δ (spec paths only): `+108 / -0`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+48` / `-0`)
- `docs/specs/table-theme.md` (`+60` / `-0`)

**Change Groups (manual classification)**

> Fill in after reading the diff hunks. Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** _(title)_
   - Buckets: _(e.g. 4,6)_
   - Confidence: _(0-1)_
   - Rationale: _(why this bucket)_
   - Evidence ledger: _(key diff fragments / terms / equations / sections)_

**Notes / Open Questions**
- _(none yet)_

</details>

<details>
<summary><strong>23. 0ccd68e0</strong> — refactor(style): improve table theme validation and add serde strictness (2026-02-04T23:43:43-05:00)</summary>

**Metadata**
- Commit: `0ccd68e0a47051b76ae46c32b8ca864f2d59de69`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-04T23:43:43-05:00
- Scope Δ (spec paths only): `+49 / -12`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+49` / `-12`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Introduce semantic IR skeleton + deterministic normalization rules (planned)
   - Buckets: 4, 6, 7
   - Confidence: 0.88
   - Rationale: Adds a renderer-facing semantic IR and a normalization procedure whose main goal is determinism and semantic equivalence (an architectural layer, plus engineering constraints that will later prevent drift/flake across inputs).
   - Evidence ledger:
     - New section: `Diagram IR + Normalization (Planned)`
     - `DiagramIr { diagram_type, direction, nodes, edges, clusters, labels, ports, style_refs, meta }`
     - Normalization rules: stable direction precedence, deterministic ordering keys, implicit node creation with warnings, port parsing, cluster membership ordering

2. **Group:** Warning taxonomy migration to structured, namespaced codes
   - Buckets: 5, 4, 6
   - Confidence: 0.83
   - Rationale: Replaces legacy screaming-snake codes with stable, namespaced ones and documents the implemented vs reserved warning taxonomy; mostly ministerial, but it also sharpens the architecture of diagnostics as structured events for panels/JSONL.
   - Evidence ledger:
     - Rename example: `MERMAID_SANITIZED` → `mermaid/sanitized/input`
     - New implemented codes table: `mermaid/unsupported/*`, `mermaid/sanitized/input`
     - Reserved codes list updated: `mermaid/limit/exceeded`, `mermaid/budget/exceeded`, `mermaid/disabled`, `mermaid/parse/error`

**Notes / Open Questions**
- The IR section is marked “Planned” here, but later commits treat IR+normalization as “Current”; we should keep these two sections consistent (either label by implementation status, or split “spec vs roadmap” explicitly).

</details>

<details>
<summary><strong>24. d2c0e330</strong> — bd-2oovu: mermaid complexity guards (2026-02-05T00:44:30-05:00)</summary>

**Metadata**
- Commit: `d2c0e33035ea913b7d7dc0bd807279ce0e8bed01`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T00:44:30-05:00
- Scope Δ (spec paths only): `+17 / -4`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+17` / `-4`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Make limit/budget warning codes consistent across degradation policy + taxonomy tables
   - Buckets: 5, 4
   - Confidence: 0.86
   - Rationale: Renames warning codes in the degradation policy to match the structured taxonomy and moves them from “reserved” to “implemented”, reducing ambiguity about what is emitted today vs later.
   - Evidence ledger:
     - Degradation policy: `MERMAID_LIMIT_EXCEEDED` → `mermaid/limit/exceeded`
     - Degradation policy: `MERMAID_BUDGET_EXCEEDED` → `mermaid/budget/exceeded`
     - Codes list: adds `mermaid/limit/exceeded`, `mermaid/budget/exceeded`
     - Taxonomy table: adds both as implemented; removes from “Reserved”

2. **Group:** Add deterministic complexity guards + degradation plan overview
   - Buckets: 7, 4, 6
   - Confidence: 0.84
   - Rationale: Defines a deterministic guard phase (complexity scoring, label limits, budget heuristics) and an explicit degradation plan, which is primarily an engineering/perf robustness feature but also a key pipeline/architecture concept.
   - Evidence ledger:
     - New section: `Complexity Guards + Degradation`
     - “Complexity score = nodes + edges + labels + clusters”
     - Deterministic degradation actions: hide labels, collapse clusters, simplify routing, reduce decoration, ASCII fallback

**Notes / Open Questions**
- “Budget estimates” are stated as heuristic but deterministic. For future auditability, we may want to specify the exact formula(s) used for the heuristic estimate once implementation stabilizes.

</details>

<details>
<summary><strong>25. 5a122d89</strong> — chore: update docs, E2E scripts, beads, and session tracking (2026-02-05T00:50:06-05:00)</summary>

**Metadata**
- Commit: `5a122d89aa0f90591be3fc2746e583541ae2d851`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T00:50:06-05:00
- Scope Δ (spec paths only): `+48 / -0`

**Files (spec paths only)**
- `docs/specs/table-theme.md` (`+48` / `-0`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Specify TableThemeSpec JSON export/import format + strict validation constraints
   - Buckets: 4, 6, 7
   - Confidence: 0.90
   - Rationale: Adds a deterministic, strict, versioned interchange format for theme specs with explicit constraints and a concrete flow, improving ergonomics and long-term safety of theme interchange (and aligning with “strict serde” expectations).
   - Evidence ledger:
     - New section: `JSON Export/Import (TableThemeSpec)`
     - “strict (`deny_unknown_fields`) and versioned”
     - Constraints: `effects` ≤ 64, `attrs` ≤ 16, gradient stops `[1,16]`, `padding`/`column_gap` `[0,8]`, `row_height` `[1,8]`, numeric bounds for speed/phase/intensity/asymmetry
     - Flow: `from_theme(&theme)` → JSON; parse JSON → `validate()` → `into_theme()`

**Notes / Open Questions**
- Forward-compatibility note: schema versioning is mentioned, but we should later specify the migration policy (e.g., “only up-convert; reject downlevel”; or “support v1..vN”).

</details>

<details>
<summary><strong>26. 19db685e</strong> — bd-yhp9: finalize upgrades and mermaid style order (2026-02-05T01:31:47-05:00)</summary>

**Metadata**
- Commit: `19db685ed820448d25da0890fb292699f39b6158`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T01:31:47-05:00
- Scope Δ (spec paths only): `+38 / -5`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+38` / `-5`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Refine Mermaid IR schema to match the “current pipeline” needs (meta/support/implicit nodes)
   - Buckets: 2, 4, 6
   - Confidence: 0.74
   - Rationale: The IR type signatures are updated (meta fields, node/edge structure, `style_ref`, `implicit`) and the normalization narrative is updated to reflect an implemented style-resolution step. This reads as a spec-catches-up-to-code change (but bucket 2 is marked with lower confidence until cross-checked against `ftui-extras`).
   - Evidence ledger:
     - `DiagramMeta { diagram_type, direction, support_level, init, theme_overrides, guard }`
     - `IrNode ... style_ref ... implicit`
     - `IrEdge ... style_ref ...`
     - Normalization rule #7: “resolve via `resolve_styles` ... (see below)”

2. **Group:** Document deterministic style resolution (supported props, precedence, linkStyle semantics, theme vars, contrast clamp)
   - Buckets: 4, 6, 7
   - Confidence: 0.86
   - Rationale: Adds a concrete, testable spec for how Mermaid styling directives are parsed/resolved and how unsafe/unsupported properties are handled, including ordering rules and a contrast clamp for readability.
   - Evidence ledger:
     - New section: `Style Resolution (Implemented)`
     - Supported keys: `fill`, `stroke`, `stroke-width`, `color`, `font-weight`, etc.
     - “Precedence (last wins): themeVariables defaults → classDef (class order) → node style”
     - Link style: `linkStyle default` + `linkStyle <idx>` by edge index in source order
     - Theme variable mapping: `primaryColor`/`primaryTextColor`/`primaryBorderColor`
     - Contrast clamp: `clamp_contrast` minimum 3.0, record `contrast-clamp`

**Notes / Open Questions**
- If we want “alien artifact” level explainability later: specify the exact contrast metric (WCAG formula details) and the clamping rule (“adjust fg only” vs “adjust fill only” vs “move both toward target ratio”).

</details>

<details>
<summary><strong>27. 5a28cbaa</strong> — Expand mermaid terminal renderer with layout engine, subgraphs, and config system (2026-02-05T03:26:18-05:00)</summary>

**Metadata**
- Commit: `5a28cbaaca95093c02bb28b23bd21871d7aaef8a`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T03:26:18-05:00
- Scope Δ (spec paths only): `+151 / -18`

**Files (spec paths only)**
- `docs/spec/mermaid-config.md` (`+151` / `-18`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Reframe Mermaid renderer as a deterministic multi-phase engine (pipeline, evidence events)
   - Buckets: 2, 4, 6, 7
   - Confidence: 0.78
   - Rationale: The spec moves from “planned / parser-only” framing to “current pipeline” framing, defining deterministic phases and evidence logging. This likely corrects previously outdated spec claims about implementation status (bucket 2, medium confidence).
   - Evidence ledger:
     - New section: `Engine Pipeline (Current)` with phases 1–6 (parse/init/validate/normalize/layout/render)
     - Evidence log events: `mermaid_prepare`, `mermaid_guard`, `mermaid_links`
     - Note: `capability_profile` is “parsed and stored for determinism” (reserved override)

2. **Group:** Specify implemented layout + routing and renderer behavior + fidelity tiers
   - Buckets: 4, 6, 7
   - Confidence: 0.85
   - Rationale: Documents the layout algorithm family (Sugiyama layered layout), deterministic routing outputs/stats, renderer draw ordering, and fidelity tier degradation behavior. This is architecture plus performance/robustness.
   - Evidence ledger:
     - New sections: `Layout + Routing (Implemented)`, `Renderer (Implemented)`, `Scale Adaptation + Fidelity Tiers`
     - Sugiyama details: rank assignment, barycenter crossing minimization, coordinate assignment, cluster boundaries
     - Renderer details: glyph mode (unicode/ASCII), render order clusters→edges→nodes→labels, viewport fitting
     - Degradation plan recorded in `MermaidGuardReport` and emitted to JSONL

3. **Group:** Update compatibility matrix + fallback policy + warning codes to match structured taxonomy
   - Buckets: 2, 5, 4, 6
   - Confidence: 0.80
   - Rationale: Aligns the documented support levels and fallback warnings with the structured warning taxonomy; also updates “outline fallback” narrative to describe its concrete behavior.
   - Evidence ledger:
     - `Compatibility Matrix (Parser-Only)` → `Compatibility Matrix (Current)`; graph/flowchart notes updated
     - Warning codes: `MERMAID_UNSUPPORTED_DIAGRAM` → `mermaid/unsupported/diagram`; `MERMAID_UNSUPPORTED_TOKEN` → `mermaid/unsupported/feature`
     - Outline tier description expanded; still uses pipeline with forced ASCII, hidden labels, collapsed clusters

4. **Group:** Make security and link behavior explicit (protocol allow/deny, strict vs lenient)
   - Buckets: 4, 6, 7
   - Confidence: 0.88
   - Rationale: Defines the hyperlink sanitization policy with deterministic outcomes and observable metrics, which is critical for safety and user trust.
   - Evidence ledger:
     - New section: `Hyperlink Policy`
     - Blocked protocols: `javascript:`, `vbscript:`, `data:`, `file:`, `blob:`
     - Strict allows: `http`, `https`, `mailto`, `tel`, relative paths
     - Blocked links emit `mermaid/sanitized/input` and excluded from link metrics

5. **Group:** Add “current API” usage snippet + troubleshooting guidance
   - Buckets: 6, 9
   - Confidence: 0.85
   - Rationale: Improves adoption by showing the actual call flow and giving users a compact playbook for width/density issues and diagnostics.
   - Evidence ledger:
     - New section: `Usage (Current API)` with `prepare` → `normalize_ast_to_ir` → `layout_diagram` → `render`
     - Troubleshooting: `max_label_chars`, `max_label_lines`, tier overrides, glyph mode, JSONL diagnostics via `FTUI_MERMAID_LOG_PATH`

**Notes / Open Questions**
- “Compatibility Matrix” lists non-graph diagrams as “parsed into AST; render path pending”; later specs add sample catalogs that include those types. We should keep the UX/spec story aligned (samples can exist, but must clearly label “parsed only” vs “rendered”).

</details>

<details>
<summary><strong>28. 8c2762d2</strong> — Fix 6 bugs found in deep review of mermaid pipeline (2026-02-05T09:48:42-05:00)</summary>

**Metadata**
- Commit: `8c2762d2e19fe14c129becc2193821e9e5bf0309`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T09:48:42-05:00
- Scope Δ (spec paths only): `+210 / -0`

**Files (spec paths only)**
- `docs/spec/mermaid-showcase.md` (`+210` / `-0`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Add Mermaid Showcase Screen UX + state model spec (layout regions, keybindings, metrics, acceptance)
   - Buckets: 4, 6, 7
   - Confidence: 0.92
   - Rationale: Introduces a highly implementable screen spec (regions, responsive layouts across terminal sizes, keybindings, state model, performance metrics, acceptance checklist), making the demo deterministic and testable.
   - Evidence ledger:
     - New file: `docs/spec/mermaid-showcase.md`
     - Layout presets: 80x24, 120x40, 200x60 with concrete region allocations
     - Keybindings: navigation, zoom/fit, layout toggles, panel toggles, fidelity/styles/wrap
     - State model: `MermaidShowcaseState` fields and render flow
     - Required metrics: parse/layout/render ms, iterations, objective score, constraint violations, fallback tier/reason
     - Acceptance checklist emphasizes spec-driven implementation parity

**Notes / Open Questions**
- The spec mentions “Objective score (if available)” and “constraint violations”. If these are meant to exist long-term, add precise definitions (what objective, what constraints) so E2E assertions remain stable and non-handwavy.

</details>

<details>
<summary><strong>29. 51e55477</strong> — Mermaid updates: samples + ER render (2026-02-05T09:58:00-05:00)</summary>

**Metadata**
- Commit: `51e55477e6ab1fe752cd86f80206f76407d14de7`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T09:58:00-05:00
- Scope Δ (spec paths only): `+44 / -0`

**Files (spec paths only)**
- `docs/spec/mermaid-showcase.md` (`+44` / `-0`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Add sample library catalog for coverage-driven demo exploration
   - Buckets: 6, 9, 7
   - Confidence: 0.87
   - Rationale: Adds a curated sample catalog that makes the demo discoverable and ensures diagram-type coverage for future test suites (including “unsupported” examples for deterministic fallback behavior).
   - Evidence ledger:
     - New section: `Sample Library Catalog (bd-2nkmi.2)`
     - Categories: Flow / Sequence / Class / State / ER / Gantt / Mindmap / Pie / Unsupported
     - Tags: dense/labels/links/cluster/stress (implied)

**Notes / Open Questions**
- Consider adding a stable “sample id” naming convention (not just human names) to make JSONL evidence logs + E2E assertions resilient to copy edits.

</details>

<details>
<summary><strong>30. 6e034e31</strong> — enhance(mermaid): improve diagram rendering and layout algorithms (2026-02-05T13:06:52-05:00)</summary>

**Metadata**
- Commit: `6e034e317a9f529f3573cfdaa1712b8a766f21f2`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-05T13:06:52-05:00
- Scope Δ (spec paths only): `+32 / -0`

**Files (spec paths only)**
- `docs/spec/mermaid-showcase.md` (`+32` / `-0`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Specify a deterministic status-log panel and event schema for mermaid showcase
   - Buckets: 4, 6, 7
   - Confidence: 0.89
   - Rationale: Defines a minimal, fixed event vocabulary and schema for deterministic logging, explicitly designed to be snapshot/E2E verifiable.
   - Evidence ledger:
     - New section: `Status Log Panel (Spec + Schema)`
     - Fixed event set: `render_start`, `render_done`, `layout_warning`, `route_warning`, `fallback_used`, `error`
     - Schema fields: `schema_version`, `ts_ms` (monotonic), sample id/name, dims, fidelity, status, message
     - “Never reorder; stable ordering for snapshot/E2E validation”

**Notes / Open Questions**
- `ts_ms` is “monotonic ms from run start”; for reproducible tests, ensure E2E harness can stub/override or assert relative ordering rather than exact values.

</details>

<details>
<summary><strong>31. 7f463211</strong> — bd-hudcn.1.8.3: PTY JSONL parsing + journey ASCII snapshots (2026-02-06T17:02:06-05:00)</summary>

**Metadata**
- Commit: `7f46321181291098aeefb662ae3c582ab7b90bf2`
- Author: Dicklesworthstone <jeff141421@gmail.com>
- Date: 2026-02-06T17:02:06-05:00
- Scope Δ (spec paths only): `+8 / -4`

**Files (spec paths only)**
- `docs/spec/mermaid-showcase.md` (`+8` / `-4`)

**Change Groups (manual classification)**

Prefer splitting by *logical change*, not by commit boundary.

1. **Group:** Clean up spec structure: move composition notes into the right section; promote “unsupported” examples into explicit categories
   - Buckets: 5, 9, 6
   - Confidence: 0.90
   - Rationale: This is mostly a readability and navigability fix: content is preserved, but reorganized so readers see composition guidance where it belongs and can reason about sample coverage by named diagram categories.
   - Evidence ledger:
     - Visual composition notes moved earlier under “Visual Composition Notes”
     - Sample catalog: `Unsupported` collapsed into explicit `GitGraph`, `Journey`, `Requirement` categories
     - Removes duplicate placement of the same bullet points at the bottom

**Notes / Open Questions**
- None.

</details>
