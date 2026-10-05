package p; typedef struct packed { logic a; logic [4:0] b; } st_t;
  localparam st_t P1 = '{a: 1'b1, b: 5'd3};
  localparam st_t P2 = '{1'b0, 5'd7};
  localparam int W = $bits(P1);
endpackage
module tb; import p::*;
  localparam st_t P3 = '{a: 1'b1, b: 5'd0};
  localparam logic [4:0] L = P1.b + 1;
  initial begin $display("DIGEST=%0d %0d %b %b %0d %0d %0d", P1.b, P2.b, P1, P2.a, W, P3.a, L); #1 $finish; end
endmodule