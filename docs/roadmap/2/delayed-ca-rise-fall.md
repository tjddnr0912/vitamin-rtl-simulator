(§2 delayed-ca-rise-fall · in Oracle splits (recorded, not chased)) (an integral delayed continuous assignment's rise / fall):
* an integral delayed continuous assignment's rise / fall — vita and iverilog take the largest delay among the changing bits (`sched/wait_fork.rs` `transition_delay`), verilator the whole-vector rule of IEEE 1800 §10.3.3 (ieee1800-2017.txt:10342-10345): T07 first drive vita = iverilog `W t=13`, verilator `t=2`
* Q1p `-1 -> 1` sign-extended into 8 bits vita = iverilog `t=21`, verilator `t=24`
* F2c, F2e3
* split (vita = iverilog)
