(§2 blk-scope-name-leak · OPEN · in Scoping / imports / block-locals) (a shadowing block-local's internal scope name shows in waves and diagnostics):
* A shadowing block-local's internal `$blk$<n>` scope names its waveform scope (`$scope module $blk$110` where iverilog writes `begin nm`) and an elaboration diagnostic's `[in …]` frame
* `%m` drops it since §4.5.611
* iverilog
* OPEN
