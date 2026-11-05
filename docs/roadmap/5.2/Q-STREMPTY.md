(§5.2 Q-STREMPTY · rank ① · source Q-NESTCAT r1 lens-diff PX1 (c/F_empty)) (an empty string literal in a string concatenation is sized as one bit):
* symptom: an empty string literal in a string concatenation is sized as one bit: `r = {"H", ""}` prints `[\x90] 1` (0x48 shifted left by one), iverilog 13.0 = sv2v = verilator 5.052 `[H] 1`
* IEEE 1800-2017 §6.16 (ieee1800-2017.txt:4530, `b = {"H",""}; // yields "H"`)
* evidence: PRE 01cac4b3 = Q-NESTCAT r2
