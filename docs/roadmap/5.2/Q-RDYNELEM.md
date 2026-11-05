(§5.2 Q-RDYNELEM · rank ① · source ENG-1b r1 lens-diff N3) (a write to a `real` dynamic-array element converts at the wrong width):
* slot: after 74
* symptom: a direct write to a `real` dynamic-array element converts at the wrong width: `rd[1] = b8 + c8` stores 300 on every backend (vl 44, the 8-bit sum)
* witness: `rd[1] = w64` stores -1.0 on native (vm / interp / vl 1.8e19), a backend split
* evidence: PRE = ENG-1b
* fix: the element store takes the integral-to-real conversion of a whole `real`
