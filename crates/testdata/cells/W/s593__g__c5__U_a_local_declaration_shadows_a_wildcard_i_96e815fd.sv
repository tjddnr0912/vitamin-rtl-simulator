package p; typedef struct packed { logic a; logic [4:0] b; } st_t; localparam st_t P = '{a: 1'b1, b: 5'd3}; endpackage
module tb; import p::*; typedef struct packed { logic [2:0] x; logic [2:0] y; } L; L P = '{x: 3'd5, y: 3'd2};
  initial begin $display("DIGEST=%0d %0d", P.x, P.y); #1 $finish; end
endmodule