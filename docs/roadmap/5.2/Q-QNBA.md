(§5.2 Q-QNBA · rank —) (a nonblocking write to a queue element is accepted):
* symptom: a nonblocking write to a queue element is accepted (IEEE 1800 §6.21 forbids a nonblocking assignment to a dynamically sized variable's element; vl refuses)
* fix: refuse by name
* source: ENG-1b r1 lens-diff N9 (BS_qelem_tern_nba, _lit_nba, _sfn_nba, _afn_nba, _cat_nba)
