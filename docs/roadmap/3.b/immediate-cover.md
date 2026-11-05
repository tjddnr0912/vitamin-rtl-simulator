(§3.b immediate-cover · SPLIT · in System tasks & file I/O) (immediate `cover` forms are E2002; both oracles accept and drop them):
* symptom: `cover (c) stmt;`, `cover #0`, `cover final` are E2002
* oracles: both oracles accept and drop the statement (§16.3 runs it)
* parse onto the assert lowering with a pass arm
* status: SPLIT
