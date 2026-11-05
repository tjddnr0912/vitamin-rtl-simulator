(§5.2 Q-SHORTREAL · rank ② · source ENG-1b r1 lens-diff N11) (`shortreal` is not supported):
* slot: §8: reopens on a target design
* symptom: `shortreal` is a parse error (E2002) in vita
* oracles: every oracle runs it
* IEEE 1800 §6.12
* fix: size the type (parse, 32-bit IEEE 754 storage, conversions) before a slice
