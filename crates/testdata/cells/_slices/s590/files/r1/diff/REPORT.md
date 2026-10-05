# §4.5.590 r1 lens DIFFERENTIAL — REPORT (live, updated per question)
round: 1
status: started
## Findings
(none yet)
## Cells
### batch 1 (o1/, 20 cells d01-d21; runner r.py: PRE2 x3, POST x3, iverilog, verilator)
- no POST backend split except pre-existing W4030 fallback line (d19, PRE2 same).
- POST == both oracles (moved from PRE2): d07 finish-in-initial, d10 port-in call, d11 port-out call, d14 force at t0.
- PRE2==POST, both == iverilog: d01 gated clock t0 edges, d02 async reset via held CA, d03 always_comb producer, d04 NBA input, d05 #0 input, d08 $strobe/$monitor, d17 always@*/always_comb reader.
- refused alike PRE2/POST (not exercised): d12 wand/tri1 (E3009), d13 assign strength (E2002), d16 seeded $random in function (E3009).
- d06 $finish in held f: PRE2 native split (Finish@3 vs Error@0) closed by POST; F4004 refusal pre-existing.
- d09 cycle through two functions: calls PRE2 7, POST 4, iverilog 3, verilator 6 (neither->neither, closer).
- d18 held chain across 6 generate instances through `wire [3:0] o[0:5]`: f calls PRE2 22, POST 18, iverilog 6, verilator 6; POST still calls f K=2..5 with x=z/x — waves collapse (per-NET preds: every link reads/drives the one array net -> treated as a cycle). Doc claim "a chain of held assigns evaluates each link once, on its settled input" false for this shape. CANDIDATE MAJOR(doc)/MINOR(residue).
- d15 VCD: POST $dumpvars block records held wires y,y2 as z then x/1 at #0; PRE2 x then 1; iverilog x / 1. End-of-#0 values identical. harness-format/observation.
- d19 delayed held chain: t0 calls PRE2 11, POST 8, iverilog 1, verilator 3 (uncertified delayed CA per-pass residue, pre-existing class).
