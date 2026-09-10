# 01 · Synthesizability legend

Every reference note in this folder marks each construct or feature with one of the following markers.

## Legend

| Marker | Meaning |
|---|---|
| ✅ | **Synthesizable** — a mainstream synthesis tool turns it into gates |
| ⚠️ | **Conditional** — only certain forms synthesize / tool-dependent / synthesizable but not recommended |
| ❌ | **Not synthesizable** — simulation and verification only (for example `class`, assertions, `wait`, dynamic memory) |

## How it is used

```
### `always_ff @(posedge clk)`
✅ Synthesizable. Synthesizes to an ordinary clocked register.

### `initial`
⚠️ Simulation construct. FPGA synthesis supports it in part (initial values); for ASIC it is
generally not synthesizable.

### `class`
❌ Not synthesizable. Verification only (the SV OOP extension).
```

## How the marker is decided

The classification is cross-referenced against three tools.

- **Synopsys Design Compiler (DC)** — the industry standard for ASIC synthesis
- **Xilinx Vivado Synthesis** — the representative FPGA synthesis tool
- **Cadence Genus** — a major ASIC synthesis tool

If at least two of the three recognize the construct as RTL and translate it, the marker is ✅. If the tools disagree, or synthesizability depends on the shape written, the marker is ⚠️. If no tool turns it into gates, the marker is ❌.

## Relationship to this project

Vitamin is a **simulator**; synthesis is a non-goal (see [01-goals-and-scope.md](../01-goals-and-scope.md)). These notes still record synthesizability for two reasons.

1. It helps a user writing real RTL recognize which forms are synthesis-friendly.
2. It names the boundary that a design intended for hardware has to stay inside, which is a different boundary from the one the simulator draws.

The markers therefore say what a synthesis tool would accept — they are not a statement about what vita accepts, and the two sets do not coincide: `initial`, `#delay`, `$display`, `$finish`, assertions, classes and randomization are all marked ❌ or ⚠️ here and are all simulated. For the construct-by-construct status of vita itself, the authority is [manual/003_language-reference.md](../../manual/003_language-reference.md); for the scope of the project, [01-goals-and-scope.md](../01-goals-and-scope.md).

## Sources

- This project's spec, §2.1 (the requirement that these reference notes mark synthesizability)
- Synopsys Design Compiler Synthesis User Guide
- Xilinx Vivado Design Suite User Guide: Synthesis (UG901)
- Cadence Genus Synthesis Solution User Guide
- IEEE 1800-2017 §A (synthesizable subset annex)
