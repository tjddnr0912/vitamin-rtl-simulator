package pk; localparam logic [3:0] E1 = 4'd7; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1, E2} e_t;
  localparam int K = E1;
  initial #1 $display("etn E1=%0d K=%0d b=%0d x=%b", E1, K, $bits(E1), {E1});
  initial #100 $finish;
endmodule
