(§5.2 Q-CONSTXIDX · rank ② · source Q-MEMBSRC re-plan cells kxi, kxiu) (a constant select whose index has an x or z bit is refused instead of reading x):
* slot: after Q-PARAM2STATE
* symptom: a constant select whose index has an x / z bit is E3009 where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12351–12353) reads x: `localparam logic [15:8] VP = 8'hA5; localparam logic X = VP[3'b0x1];` prints "parameter `X` value is not a constant: 3'b0x1 has no constant-fold arm", iverilog 13.0 `x`
* witness: `localparam bit X = $isunknown(VP[3'b0x1]);` the same E3009, iverilog `1`
* witness: the member twin `P.a[3'b0x1]` identical
* evidence: PRE 9d60771d
* status: loud
