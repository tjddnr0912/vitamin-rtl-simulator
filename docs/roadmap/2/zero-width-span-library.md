(§2 zero-width-span-library · RECORD · in Ranges / bounds / selects) (library-mode spans could fool row 65's zero-width check (unreachable today)):
* The row-65 zero-width claim compares a width's span with `cur_module_span` (`edge_gate.rs` `zero_width_claim_is_sound`), and in `velab -L` library mode the spans of different compilation units share one coordinate space (manual 004 §15.6), so text from another unit could fall inside the module's range
* not reachable today (library mode loads neither a package nor a `$unit` routine from another unit; a child module in another unit is claimed right) (row 65 review round 2, differential N2)
* RECORD
