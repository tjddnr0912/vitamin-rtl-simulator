(§3.b frame-concat-split-residue · OPEN · in Subroutine / frame) (concatenation targets over frame locals with a delay, call or hierarchy):
* symptom: in a frame body, a concatenation target over a frame local with an intra-assignment delay or event (`{m, junk} = #1 x;`), an index that reads a module net or calls (`{n, junk[sel]} = e;`), or a hierarchical target is E3009 (`frames_concat.rs` declines, `frames_classify.rs` arm 5 keeps a lifted task loud; PRE wrote no local there, `y=0` at exit 0)
* split after the timing (`tmp = e; #d;` then the writes) and admit a non-target module-net index read once a hierarchical spelling of a target is resolved to its net
* oracles: iverilog, sv2v and verilator `y=6`, `y=2`
* status: OPEN
