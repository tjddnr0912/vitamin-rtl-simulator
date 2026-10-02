# The vitamin history directory

`docs/history/` is a frozen pointer. A slice's record is its commit message; this directory does
not grow with slices, and nothing in it describes HEAD.

## What is here

| Path | Contains | Answers |
|---|---|---|
| [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) | A pointer file: the `git show` commands that print the completed-slice records (`§4.5.N`, `§5.1-<id>`), and `git log --grep` for slices from `§4.5.583` on | Where a `§4.5.N` or `§5.1-<id>` citation in a code comment or commit message resolves |
| [reviews/](reviews/) | One review document received from outside (2026-07-02), preserved verbatim. Its internal relative links point at the reviewer's own repository and do not resolve here | What was asked for, in the requester's own words, before it became a specification |
| [research-log/](research-log/) | 24 files: 22 primary-source research rounds behind the language reference notes, a README stating the naming and front-matter convention, and `METHODOLOGY.md`, the multi-round research method. Each note carries its queries and the URLs fetched | Which sources a claim in [../preview/hdl-reference/](../preview/hdl-reference/) rests on |

## The reading rule

A statement in a file here is true of its date, not of HEAD. A search hit inside `docs/history/`
answers "what was believed on that date", never "what the simulator does".

## Where the current answers are

| Question | Document |
|---|---|
| What is under `docs/` | [../README.md](../README.md) |
| How to install, run, and what is supported | [../manual/](../manual/) |
| What is still open | [../ROADMAP.md](../ROADMAP.md), [../REMAINING_WORK.md](../REMAINING_WORK.md) |
| How work is done here | [../ENGINEERING_RULES.md](../ENGINEERING_RULES.md) |
| The design contract | [../preview/](../preview/) |
| What changed between releases | [../../CHANGELOG.md](../../CHANGELOG.md) |
