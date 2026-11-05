(§5.2 Q-DEASSIGNCA · rank ① · source Q-RELDELAY r1 lens-diff F6, r2 sketch (M37 M37c)) (`deassign` of a variable a continuous assign drives is accepted):
* symptom: `deassign` of a variable that a continuous assignment drives is accepted: M37 (`assign #3 v = a;`, `force v`, `deassign v`) runs on vita
* oracles: iverilog and sv2v refuse (`Cannot perform procedural assignment to variable 'v' because it is also continuously assigned.`), verilator `%Error-CONTASSINIT`
* oracles: the three read IEEE 1800-2017 §6.5 (no mixture of procedural and continuous assignments to a variable) as covering `deassign`
* fix: refuse by name
* pins: M37 is the one killer of Q-RELDELAY's mutant M5 (the release's write of a still-forced chunk): once this refuses, M37's pin turns loud and M5 equivalent
