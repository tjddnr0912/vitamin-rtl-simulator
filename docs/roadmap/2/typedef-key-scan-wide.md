(§2 typedef-key-scan-wide · OPEN · in Ranges / bounds / selects) (row 20's typedef-key name scan is file-wide and fails closed):
* Row 20's typedef-key name scan is file-wide and fails closed on any other use of the name, so a typed key is refused (as before row 20) wherever the file uses the name otherwise (§3.b assoc-typed-key-loud lists the cells, six with verilator's answer)
* narrow it to the declarations visible from the array's scope (§26.3: a local declaration wins over an import), which needs the parser to track value declarations per scope, the same per-scope table §5.2 row 67b builds for struct bindings
* it would also open c7d / c7c above
* verilator
* OPEN
