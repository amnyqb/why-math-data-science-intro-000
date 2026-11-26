# Inbox

Messages for the **builder** (the assistant that changes Dynamist). The builder reads this page at the start of every round, answers under each message, and moves finished items to *Handled*.

## How to leave a message

Copy this block to the end of **Open messages** and fill it in:

```markdown
### YYYY-MM-DD — short title
**From:** who is writing (e.g. Claude Desktop, on behalf of Amin)
**Kind:** model request · question · bug · feature · review notes
**Files:** files you put in `inbox/` (or full paths)
**Message:**
What you want, in plain words. For a model: what the system is, what questions the model should answer,
the time span and time unit, and anything you're unsure about.

**Status:** open
```

The builder replies right under the message with **Reply (date):** and changes **Status** to `in progress`, `needs answer` or `done`.

Writing from **another project or conversation**? Read [Other projects](Other-Projects.md); the builder answers here and in [OUTBOX.md](OUTBOX.md).

## Tips for model requests

- Structure the workbook with the [Spreadsheet template](Spreadsheet-Template.md) — it saves a lot of back-and-forth.
- Keep the original workbook too (put both in `inbox/`): the builder can check numbers against it.
- List questions the model should answer ("does running out happen before day 200 if demand grows 5%?") — they decide what goes on the dashboard and into scenarios.

---

## Open messages

### 2026-11-18 — Fuel Supply Planning Simulator
**From:** Amin · **Kind:** model request
**Files:** `Fuel Supply Planning Simulator.xlsx` (Downloads), restructured to `inbox/fuel-supply/` and `inbox/Fuel-Supply-Model.xlsx`
**Message:** Demonstrate the app with this workbook: stock-and-flow, causal loop diagram, all details, grouping/abstraction, and arrays-like repetition without jargon.
**Reply (2026-11-18):** Done — see [Fuel supply example](Fuel-Supply-Example.md) and [Sets](Sets.md). Open `inbox/fuel-supply` with File → Import model from spreadsheet… (choose the folder) or the `.xlsx`. Open questions for you: (1) the 8 tanks marked not ready (Qassim, Qassim 2, PP11, PP12, PP14, Marifq IWPP, Rabigh Steam, Taiba 1) lack capacity, max stock or stock — send them if a plan is wanted; (2) should expiry/loading performance (the Summary sheets) be modelled as a second model?
**Status:** needs answer

### 2026-11-23 — Round 8: sd-ai benchmark, why results changed, audit pack, blind hindcast
**From:** Amin · **Kind:** feature
**Files:** [inbox/round-8-prompt.md](inbox/round-8-prompt.md)
**Message:** B73 to B77 as written in the prompt.
**Reply (2026-11-23), branch `round-8`, tests green on the server (374 passed, 0 failed, 4 ignored, whole workspace at `bb3f250`):**
- **Done:** B75 `dyn attribute`, B77 `dyn hindcast`. Documented in [Attribute, pack, hindcast](Attribute-Pack-Hindcast.md).
- **Not finished, B76:** `dyn pack` makes every part except `reporting-checklist.md`. The open copy of Rahmandad and Sterman (2012) answered 429 (retry after 900 seconds) three times, so no item could be read, and nothing was written from memory. It is in `papers/TO_FETCH.md`. If you put the PDF in `papers/`, the checklist can be built from it.
- **Not finished, B73 and B74:** the tools, the flag, their tests and the dev runs are done; see `benchmarks/sdai/REPORT.md`. The work is a branch of a local clone of sd-ai outside this repository (no fork on GitHub was made; that is your step). What is missing, and why:
  1. **formulationCritique was re-graded with DeepSeek as the judge** (deepseek-v4-flash), because sd-ai's Gemini judge was out of free-tier quota. Numbers are not comparable with sd-ai's published ones. Plain formulationCritique 58.3% in both arms; hard 77.1% without the tools, 75.0% with.
  2. **The tools did not help on any hard group.** extremeConditionsHard: 72.2% without the tools, 66.7% with them (interval for the difference -25.8 to +15.3 points); every interval includes zero. The agents used a tool in 53 of 78 runs; on the hard group they ran fewer what-if runs with the tools (0.58 per run against 1.25), and the grader credits only what-if runs.
  3. **Not run:** Opus 5.5 (needs your approval of the cost; a Sonnet pass of both arms cost about $30 in all), the held-out groups (never opened), the leaderboard entry.
  4. Total spent on API calls: about $32.4 (dev runs $30.08, smoke tests $2.35). No single run came near $50.
- **Tag:** `b74-freeze` marks the Dynamist state the dev runs used; the held-out run and any rule change are your decision.
**Status:** needs answer

---

## Handled

### 2026-11-18 — Wiki for working together
**From:** Amin · **Kind:** feature
**Message:** Build a wiki with full documentation and a feature document with a checklist, logging and history, so another assistant can structure a complex Excel workbook in a way that's easy to communicate.
**Reply (2026-11-18):** Done — this wiki. Start with [Spreadsheet template](Spreadsheet-Template.md); records are in [FEATURES.md](FEATURES.md), [CHANGELOG.md](CHANGELOG.md) and [DEVLOG.md](DEVLOG.md). Put the workbook in `inbox/` and leave a message above.
**Status:** done
