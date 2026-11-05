(§3.b class-field-select · OPEN · in Strings / heap) (a part-select of a class field is refused):
* symptom: `c.u8[3:0]` is E3010
* route the select's base through the class-field read
* oracles: verilator
* status: OPEN
