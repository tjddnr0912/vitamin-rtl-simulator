(§2 signed-string-default · OPEN · in Index sealing) (`signed` on a string default is dropped):
* `signed` on a string default is dropped (`str_param_raw`)
* 2 oracles
* OPEN
* note: the keyword is recorded since §4.5.635 (`ParamDecl` dtype.signing)
* the string route (`param_is_untyped`, `str_param_raw`) still ignores it on purpose, pinned by `cli/tests/param_default_takes_declared_type.rs` an_untyped_string_default_is_unchanged, so this item's fix reads the record
