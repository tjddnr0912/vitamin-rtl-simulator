(§3.b cli-lib · OPEN · in Diagnostics quality) (the cli crate's library tests fail to build without default features):
* symptom: `cargo test -p cli --no-default-features --lib` fails E0004 (a dev-dependency revives sim-engine's `oracle`)
* `default-features = false` on it
* CI cannot see it
* status: OPEN
