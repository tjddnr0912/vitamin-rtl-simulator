(§3.b mdrv-ca-array · RECORD · in Constants / parameters) (a whole-array `assign` beside another writer gets the wrong error code):
* symptom: an unpacked array's whole-array `assign` beside another writer is E3009 (`cont_array.rs` sole-writer check), not E3001
* Rule D skips unpacked variables
* oracles: both oracles reject
* status: RECORD
