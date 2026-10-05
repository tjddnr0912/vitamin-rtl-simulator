package p; typedef enum logic [1:0] {A=1, B=2, C=3} e_t; localparam e_t P = C; e_t v = B; endpackage
module tb; import p::*;
  initial begin $display("DIGEST=%s %s %0d", P.name(), v.name(), P.next()); #1 $finish; end
endmodule