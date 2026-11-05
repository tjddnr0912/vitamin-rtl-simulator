(§2 replication-count-cap · RECORD · in Diagnostics / artifacts) (a replication count above 2^24 repeats 2^24 times):
* A replication count above 2^24 repeats 2^24 times (`sim_ir::selfwidth::WIDTH_MAX`; `{(32'd0-32'd1){1'b1}}`): an assignment's low bits are right (pin `an_unsigned_wrap_count_still_replicates`), the replication's own width is not
* RECORD
