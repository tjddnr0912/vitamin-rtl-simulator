# HDL Reference

This folder is a per-topic reference to the syntax, the packages and the synthesizability of Verilog, SystemVerilog and VHDL. It documents the languages, not this simulator: it is consulted while implementing, to check what the standard actually requires. For what vita itself accepts, refuses or simplifies, the authority is [manual/003_language-reference.md](../../manual/003_language-reference.md).

## Folder layout

| Folder/file | Description |
|---|---|
| [00-standards-map.md](00-standards-map.md) | IEEE 1800/1364/1076/1164 editions and how they relate |
| [01-synthesizability-legend.md](01-synthesizability-legend.md) | The ✅/⚠️/❌ synthesizability markers (shared by every note here) |
| [system-tasks/](system-tasks/) | The standard `$` system tasks and functions, by category |
| [verilog/](verilog/) | Verilog (IEEE 1364) syntax and structure |
| [systemverilog/](systemverilog/) | SystemVerilog (IEEE 1800) extensions |
| [vhdl/](vhdl/) | VHDL (IEEE 1076) syntax and packages |

## Suggested reading order

1. `00-standards-map` → `01-synthesizability-legend` (the conventions used throughout)
2. `system-tasks/00-index` (coverage matrix)
3. The `00-index` of the language folder you care about → the per-topic notes

## Sources

- This project's spec, §10 (structure) — the spec is `docs/preview/`
