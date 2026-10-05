package p; typedef struct packed { logic a; logic [4:0] b; } st_t; localparam st_t P = '{a: 1'b1, b: 5'd3}; localparam st_t Q = '{a: 1'b0, b: 5'd9}; endpackage
module tb; import p::st_t; import p::P;
  initial begin $display("DIGEST=%0d %0d", P.b, P.a); #1 $finish; end
endmodule