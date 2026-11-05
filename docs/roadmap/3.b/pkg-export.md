(§3.b pkg-export · OPEN · in Parser accept) (a package `export p::name;` / `export *::*;` is rejected):
* symptom: a package `export p::name;` / `export *::*;` is E2002 (`expected module item, found '::'`; `spi_device_pkg.sv:425`)
* IEEE 1800-2017 §26.6 offers an imported name only when the package exports it
* parse it and add the exported imports to the package's offer by the origin its import arm records (one binding per key, §4.5.591)
* all three `A E=8 D=7`
* witness: row-9 R19
* status: OPEN (§5.2 row 39)
