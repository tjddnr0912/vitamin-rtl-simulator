(§5.2 Q-STATICFRAME-WAIT · rank ① · source ENG-2 r4 re-plan M1 (SKWf, H3dw)) (a static recursive task waiting on its own variable misses other activations):
* symptom: a static recursive (framed) task whose `@(*)` reads its own variable while a second activation writes it runs silent-wrong: SKWf (two processes call it) `A end y=x`, iv = s2v `W t=3 y=5 \| W t=3 y=5 \| A end y=5`
* witness: H3dw `A end y=0`, iv = s2v `y=9`
* status: pre-existing (PRE = POST)
* IEEE 1800-2017 §13.3.2 (one variable per instance shared by every activation)
* fix: needs an activation census
