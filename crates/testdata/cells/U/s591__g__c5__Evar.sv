package pk; logic [7:0] E1 = 8'd5; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
  wire [3:0] r = {E1{1'b1}};
  initial #1 $display("evar E1=%0d K=%0d r=%b pk=%0d", E1, K, r, pk::E1);
  initial #100 $finish;
endmodule
