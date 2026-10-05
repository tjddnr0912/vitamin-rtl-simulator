package p; typedef struct packed { logic a; logic [4:0] b; } st_t; localparam st_t P = '{a: 1'b1, b: 5'd3}; endpackage
module tb; import p::*;
  localparam int W = P.b + 1; logic [W-1:0] v = '1;
  initial begin $display("DIGEST=%0d %0d %0d", W, v, $bits(v)); #1 $finish; end
endmodule