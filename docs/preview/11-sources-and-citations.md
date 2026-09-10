# 11 · Sources, copyright and citation

The rules for using outside material in this repository, and the catalogue of what the
reference notes are built from. This is a public repository under a permissive licence, so
what may be copied in, and how it must be attributed, is a contract rather than a courtesy.

The material this governs is the standards-derived reference set — everything under
[hdl-reference/](hdl-reference/) and the raw research behind it — plus any outside text
quoted in a specification. It does not govern the source code, which carries its own licence
headers.

---

## 1. What may be reproduced

| Material | Rule |
|---|---|
| IEEE standards (1364, 1800, 1076, 1164) | Copyrighted and sold. Summarise, paraphrase and cite by clause. Do not reproduce the text verbatim, and do not reproduce the formal BNF as printed. |
| Freely licensed material (open tool documentation, public grammars, university course notes) | Quote with attribution, subject to whatever licence it carries. |
| Commercial or paywalled material (vendor manuals, subscription references) | Cite by title and clause. Quote only within fair-quotation limits. |
| Anything under a permissive open-source licence copied into the tree | Bring its licence file with it and keep the original attribution intact. |

The IEEE GET Program distributes some of these standards at no charge — IEEE 1800-2017 and
1800-2023, and IEEE 1076-2019. Free access is a reading right, not a redistribution right;
the rule above does not relax for a standard obtained that way.

Free material is preferred wherever it answers the question. When a paid standard and a free
secondary source disagree, the standard decides and the disagreement is recorded rather than
smoothed over.

---

## 2. How to cite

Citations sit in a `## Sources` section at the foot of the document. One entry per line:

```
- <what it establishes>: <URL or standard number>, accessed YYYY-MM-DD
```

| Rule | Form |
|---|---|
| Name a standard by its full number, including the year | `IEEE 1800-2017`, `IEEE 1364-2005` — never bare `IEEE 1800` in a Sources entry |
| Cite a clause with the section sign | `IEEE 1800-2017 §4.4.2` |
| Cite a range with an en dash | `IEEE 1364-2005 §4.1–§4.4` |
| Say what the source establishes, not just that it exists | `identifier-code encoding` beats `VCD reference` |
| Record the access date for anything fetched from the web | pages move; the date is what makes a dead link diagnosable |

Where a document's claims come from measurement rather than from literature, the equivalent
obligation is to name what produced the number — the design, the tool version and the
command — in a table beside the claim. That is the form the measurement documents use, and
it satisfies this section in place of a `## Sources` list.

Status at HEAD:

| Document set | Carries `## Sources` |
|---|---|
| [hdl-reference/](hdl-reference/) — all 49 notes | yes |
| Numbered specifications `00`–`11` and `13`–`17` | yes |
| Numbered specifications `18`–`21` | no — these are measurement documents and cite their sources inline, in per-claim tables |
| [../manual/](../manual/) and [../study/](../study/) | no — both cite inline and close with a related-documents list instead |

---

## 3. Where the research lives

The reference notes are summaries. The raw multi-round research they were written from is
kept so that a claim can be traced back to the page it came from.

| What | Where |
|---|---|
| The research notes themselves, one file per topic, named `<topic-slug>-YYYY-MM-DD.md` | [../history/research-log/](../history/research-log/) |
| The method that produced them — multi-angle search, primary-source fetch, the gap checklist, the conflict rule | [../history/research-log/METHODOLOGY.md](../history/research-log/METHODOLOGY.md) |
| The convention each note follows — front-matter block, narrative body, `## Sources` foot | [../history/research-log/README.md](../history/research-log/README.md) |

The research log is history: it records what was found at the time it was written, and it is
not updated. A statement in it is evidence about a source, never a statement about this
simulator. For that, use [../manual/](../manual/).

Re-researching a topic adds a new file; the earlier one stays, so that the sequence of
rounds remains readable.

---

## 4. The source catalogue

### 4.1 Standards

| Standard | Subject | Role here |
|---|---|---|
| IEEE 1800-2017 | SystemVerilog LRM | The baseline the front end targets. Subsumes Verilog. |
| IEEE 1800-2023 | SystemVerilog LRM, latest | Consulted for later-revision questions. |
| IEEE 1364-2005 | Verilog LRM, final standalone edition | Cited for constructs whose clause numbering readers still know from 1364, and for VCD. |
| IEEE 1076-2008 | VHDL LRM | Reference only. VHDL is documented, not implemented. |
| IEEE 1164 | `std_logic_1164` nine-value logic | Reference only; absorbed into 1076-2008. |

The version relationships, and which edition absorbed which, are laid out in
[hdl-reference/00-standards-map.md](hdl-reference/00-standards-map.md).

### 4.2 Community references

Secondary sources, used for worked examples and for cross-checking a reading of the
standard. None of them is authoritative against the standard text.

| Source | Used for |
|---|---|
| chipverify.com | Verilog and SystemVerilog worked examples |
| asicworld.com | Verilog constructs and testbench idioms |
| verificationacademy.com | Scheduling and assertion discussions |
| verificationguide.com, vlsiverify.com | Region and scheduling summaries |
| hdlbits.01xz.net | Small self-checking exercises |
| zipcpu.com | Implementation-side write-ups, notably VCD generation |
| en.wikipedia.org | Standard version history and format overviews |
| Accellera published papers | Event scheduling and language committee material |

### 4.3 Tool documentation

| Tool | Documentation | Role here |
|---|---|---|
| Icarus Verilog | https://steveicarus.github.io/iverilog/ | The live differential oracle |
| Verilator | https://verilator.org/guide/latest/ | Second opinion on 2-state arithmetic, width and sign |
| GTKWave | https://gtkwave.sourceforge.net/ | FST format and waveform viewing |
| Surfer | https://surfer-project.org/ | FST reading, via the `wellen` parser |
| Yosys | https://yosyshq.readthedocs.io/ | Synthesis-side behaviour, for questions about what is synthesisable |

### 4.4 Source repositories read directly

Where documentation was ambiguous, the implementation was read.

| Repository | Read for |
|---|---|
| github.com/steveicarus/iverilog | Observed behaviour behind an oracle divergence |
| github.com/verilator/verilator | 2-state semantics and width handling |
| github.com/YosysHQ/yosys | Synthesis subset boundaries |
| github.com/dalance/sv-parser | SystemVerilog grammar coverage |
| github.com/gtkwave/gtkwave | FST container layout |

---

## 5. Third-party material inside this repository

| Material | Location | Licence and handling |
|---|---|---|
| This project | whole tree | MIT OR Apache-2.0, at the user's option. [../../LICENSE-MIT](../../LICENSE-MIT), [../../LICENSE-APACHE](../../LICENSE-APACHE). |
| Vendored `libm` 0.2.16 | `third_party/libm/` | MIT, upstream licence kept verbatim at [../../third_party/libm/LICENSE.txt](../../third_party/libm/LICENSE.txt). Vendored so that the real-math system functions produce bit-identical results on every platform. It is a path dependency but deliberately not a workspace member, so workspace-wide lints do not rewrite third-party code. |
| Workload corpus RTL | `bench/` | Not redistributed. The manifest pins an upstream commit per workload and the runner clones it; only permissive licences are admitted — MIT, BSD-2-Clause, BSD-3-Clause, ISC, Apache-2.0. Per-workload licence and pinned revision: [../study/03-workload-corpus.md](../study/03-workload-corpus.md), reconstruction recipes: [../../bench/README.md](../../bench/README.md). |
| First-party corpus designs | `bench/keccak/` | Written for this project and covered by this repository's licence. |

---

## 6. When the policy is breached

Verbatim reproduction of copyrighted text is replaced with a summary in its own commit, and
that commit's message opens with `Fix copyright:` and names what was replaced. The
attribution stays: removing the offending text does not remove the obligation to say where
the idea came from.

An unattributed claim found in a reference note is treated the same way as an unmeasured
claim in a specification — it is either sourced or removed, never left standing on the
grounds that it reads correctly.

---

## Sources

- IEEE 1800-2017, IEEE 1800-2023, IEEE 1364-2005, IEEE 1076-2008, IEEE 1164: standard identification and clause-citation form.
- IEEE GET Program, free-access terms: https://standards.ieee.org/products-programs/ieee-get-program/
- Accellera downloads and published papers: https://www.accellera.org/downloads/ieee
- Icarus Verilog documentation: https://steveicarus.github.io/iverilog/
- Verilator documentation: https://verilator.org/guide/latest/
- GTKWave documentation: https://gtkwave.sourceforge.net/
- Vendored `libm` upstream licence: [../../third_party/libm/LICENSE.txt](../../third_party/libm/LICENSE.txt)
- Research method and the per-topic source lists: [../history/research-log/METHODOLOGY.md](../history/research-log/METHODOLOGY.md), [../history/research-log/](../history/research-log/)
- Terms used above: [10-glossary.md](10-glossary.md)
