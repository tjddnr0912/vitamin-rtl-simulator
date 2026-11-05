(§3.b cover-immediate · OPEN · in Parser accept) (`cover` in a procedure is E2002 and at module level gets the assert message):
* symptom: `cover (c) stmt;`, `cover #0` and `cover final` are E2002 in a procedure, and as a module item they get the `assert`/`assume` message (`expected property after assert/assume at module level`)
* IEEE 1800-2017 §16.4 makes the deferred cover a module item too
* the procedural form first, then the item takes §4.5.606's desugar
* witness: no corpus witness
* status: OPEN
