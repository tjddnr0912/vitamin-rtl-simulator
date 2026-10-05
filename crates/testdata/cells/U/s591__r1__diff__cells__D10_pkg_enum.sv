package pk; typedef enum logic [1:0] {A, B, C} e_t; localparam P = 2; endpackage
module top;
  import pk::*;
  import pk::B;
  e_t v = C;
  initial #1 $display("d10 B=%0d v=%0d P=%0d", B, v, P);
  initial #100 $finish;
endmodule
