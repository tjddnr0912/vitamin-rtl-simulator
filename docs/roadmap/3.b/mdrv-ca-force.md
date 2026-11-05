(§3.b mdrv-ca-force · RECORD · in Constants / parameters) (`force` over a variable's continuous `assign` is no second driver):
* `force` over a variable's continuous `assign` is not a Rule D writer (`ProcWrites::ExceptForce`, IEEE §10.6.2)
* oracles: iverilog and verilator run it `y=1 y=0 y=1 y=0` = vita
* status: RECORD
