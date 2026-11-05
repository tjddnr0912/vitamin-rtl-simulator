(§5.2 Q-DLYCAT · rank ② · source row 26 r1 S3 (p2g, p2f)) (an intra-assignment delay on a string concatenation is refused):
* slot: PARKED 2026-10-30 behind Q-CAPLIFE and Q-ALWINTRA, W/qnestcat/RULINGS-replan2.txt
* slot: Q-ALWINTRA itself PARKED 2026-11-02 behind Q-ALWBUDGET, so two steps
* an intra-assignment delay on a string concatenation is refused: `r = #1 {t, "!"}` and `r = #1 ({t, "!"})` are E3009 "an intra-assignment delay on a string concatenation is unsupported" (`elaborate/src/strings.rs:893–899`, `string_concat_special`), iverilog 13.0 = sv2v = verilator 5.052 `[abc!] 4 t=1`
* the generic lowering is already right (`r = #1 string'({t, "!"})` = all three)
* IEEE 1800-2017 §9.4.5 (ieee1800-2017.txt:9743–9757)
* PRE 9d60771d
* loud
* PARKED after three blocking rounds on one axis, the capture a delayed statement shares between overlapping activations (CAND1 Y1k / Y2k; CAND2 F1; C3A2 G1, a library-mode `span.lo` key collision, with G2 and G3): every delayed concatenation with a direct `string` operand keeps PRE's refusal
* measured starting points: option (a) (C3A2 = C3A) opened 52 cells / 116 lines (W/qnestcat/r1/parked_dlycat.tsv), option (b) leaves 76 cells / 160 lines loud
* re-derived from measurement after Q-CAPLIFE and Q-ALWINTRA
* source: Q-NESTCAT design reviews r1–r2 (W/qnestcat/review-design2/REPORT.txt)
