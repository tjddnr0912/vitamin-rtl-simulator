(§3.b typedef-atom-cast · OPEN · in System tasks & file I/O) (a cast to an atom or `bit` typedef is refused):
* symptom: `ti'(…)` to an atom / `bit` typedef is E3009 (`simple_typedef_cast`)
* desugar to `CastTarget::Prim`, or a size cast + 2-state coercion
* oracles: 2 oracles
* status: OPEN
