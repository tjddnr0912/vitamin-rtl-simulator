module tb; typedef struct { logic a; logic [4:0] b; } us_t; localparam us_t U = '{a: 1'b1, b: 5'd3};
  initial begin $display("DIGEST=%0d", U.b); #1 $finish; end endmodule