(§5.2 Q-PKCAP · rank — · source Q-XFALLBACK plan P8 (a), review dr2 N-4) (the packed dimension product has no net-width cap (memory blow-up)):
* slot: a crash: D10 machine hazard
* slot: `packed.rs`: D12 file check at start, Q-IDXVALUE and Q-MDCONSTOFF edit it
* T′ — `packed_extents` (`elaborate/src/packed.rs:181–192`) multiplies a packed declaration's dimensions with no net-width cap where the other geometry sinks cap at 2^20: `logic [1:0][33'h1_0000_0005:0] d;` (no x) grows past 1 GB of resident memory, and `logic [1:0][((X << 32) + 5):0] d` (X = `16'd1 % ('1 + 1'b1)`) was OOM-killed at a generate-block and an instance binder and prints `V n=-1` at a module after 25 s and 5 GB, PRE and §4.5.632 alike (W/qxfb/probe/pkcap, Q-XFALLBACK review dr2 N-4: shl32__PK__G / I / M, excluded from that row's cell runs)
* fix: the packed product takes the same cap (and message) as the unpacked and net-width sinks before any allocation
* exit: the three PK cells refused in milliseconds, the 3,059 cells, the corpus
* pre-existing
