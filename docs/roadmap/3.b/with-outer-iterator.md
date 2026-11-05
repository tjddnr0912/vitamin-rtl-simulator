(§3.b with-outer-iterator · OPEN) (a nested `with` clause cannot read the outer iterator):
* symptom: a nested `with` clause cannot read the outer iterator: `rd.sum(x) with (id.sum(y) with (y * x))` (rd = '{1.5, 2.5}, id = '{1, 2}) is E3010 `undeclared net/variable top.x` (PRE 01cac4b3), where IEEE 1800-2017 §7.12 (ieee1800-2017.txt:7089: the iterator's scope is the with expression, which holds the inner clause) and §7.12.3 (:7199–7200, both sums real) give 12.0
* oracles: verilator 5.052 9.000000 (it types a with-sum by its receiver, PROBE_CATALOG)
* oracles: hand
* status: OPEN
