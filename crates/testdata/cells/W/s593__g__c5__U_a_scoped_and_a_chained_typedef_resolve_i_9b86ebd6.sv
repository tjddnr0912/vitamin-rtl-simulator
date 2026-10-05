package p; typedef logic [5:0] u; typedef struct packed { logic a; logic [4:0] b; } st_t; endpackage
module tb;
  localparam p::u Q = 6'd9;
  localparam p::st_t R = '{a: 1'b0, b: 5'd2};
  typedef p::u u2;
  localparam u2 S = 9'h1FF;
  initial begin $display("DIGEST=%0d %0d %0d %0d", Q, R.b, S, $bits(S)); #1 $finish; end
endmodule