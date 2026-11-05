(§5.2 Q-DSV · rank ① · source ENG-1 r2 (C13, P08; real / string first tick H20 H23 C14 C15 once ENG-1b lands)) (an SVA past-value register starts at the type default, not the declared value):
* symptom: an SVA previous-value register (`$past` / `$stable` / `$changed` / `$rose` / `$fell`) starts at the type default instead of the signal's declaration value: `int r = 5; assert property (@(posedge clk) $stable(r))` fails at the first clocking event (verilator passes)
* IEEE 1800 §16.5.1 default sampled value, §16.9.3
* fix: init the register from the static variable's declaration value (elaborate only)
* note: size S
