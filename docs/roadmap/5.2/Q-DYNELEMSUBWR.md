(§5.2 Q-DYNELEMSUBWR · rank ① · source row 24 r4 lens-sound N1, r4 re-plan) (a sub-select write into an associative-array or queue element is dropped):
* symptom: a sub-select write into an associative-array or queue element is dropped with W4020 at exit 0, in the literal member and the plain spellings (row 24 r4 cells aa_l, aa_p, aa_whole, q_lr, q_pr: `m[5].a[12:9] = 4'hF` on `h_t m[int]` leaves `A m5=5a7`)
* IEEE 1800-2017 writes the bits
* status: silent-wrong (warned), pre-existing
