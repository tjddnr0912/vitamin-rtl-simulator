(§5.2 Q-PKGPMEMB · rank ② · source Q-MEMBSRC plan probe pkgm) (a member of a package-scoped struct parameter is refused at parse):
* slot: after Q-MEMBSRC
* symptom: a member of a package-scoped struct parameter is E2002 at parse: `pk::P.a`, `pk::P.a[9]` ("expected ';', found '.'"), also in a parameter default
* oracles: sv2v → iverilog `a5 0`
* oracles: iverilog "Parameter name P can't have member names (a)." and an assertion
* evidence: PRE 9d60771d
* status: loud
