(§2 velab-mtime-stamp · RECORD · in Diagnostics / artifacts) (a work-library `.velab` stores unit mtimes, so its bytes differ run to run):
* witness: w2p32 through the work library, `vcmp --work lib` then `velab -L lib --top top`, in two fresh directories on PRE d59629fa: the `lib` files are identical (1efdf4c2), `t.velab` is 425f791b in one and 9fd9a72c in the other, 15 bytes apart at offsets 559–574 beside the embedded unit path `./lib/units/cu_….vu`
* witness: in w2p32 a copied `lib` (new mtimes) gives a third `t.velab`, f99fbf9b (W/main_d7/2026-11-08/lens4diff, W/qvalrulesd/RULINGS-lens4.txt differential NOTE (4))
* cause: by design: the 15th `.velab` trailer (RULEV-MTIME) stores a verified `(mtime, size)` stamp per consumed work-library entry so `vrun` can skip a rehash (`crates/cli/src/lib.rs:612-625`, `stamp_verified` at `crates/cli/src/pipeline.rs:825`)
* note: CONTRIBUTING's determinism rules promise byte-identical output across Linux and macOS and ER §9 keeps artifacts byte-identical across platforms, but no rule or test says whether a work-library `.velab` is a byte-reproducible artifact; an explicit-path `.velab` carries the trailer empty (`crates/cli/src/lib.rs:624-625`)
* note: a harness comparing work-library `.velab` files by md5 sees a difference from the stamp alone, so it compares the outputs or masks the 15th trailer
* RECORD
