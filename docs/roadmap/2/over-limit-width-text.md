(§2 over-limit-width-text · OPEN · in Diagnostics / artifacts) (an over-limit width refusal prints a saturated width):
* symptom: the over-limit refusal at the two existing edge refusals prints the part-select's width saturated at u64, not its true width: on `v[W:0]` with W = 64'hFFFF_FFFF_FFFF_FFFF it prints "(18446744073709551615 bits)" where the select is 2^64 bits wide (part_width saturation); PRE's, unchanged by Q-LOSSYSPELL's fix H (W/lossy/fixh/premise4/REPORT.txt F5, cell pc_r10; W/batch6/RULINGS.txt L2-55)
* note: a refusal's number is a claim of the diagnostic (IEEE 1800-2017 §6.9.1, l.4294-4295, lets the implementation set the limit; the text should state the width it refused)
* status: OPEN
