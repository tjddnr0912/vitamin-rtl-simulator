package pk; localparam E1 = 7; endpackage
typedef enum {E0, E1} e_t;
module top;
  import pk::E1;
  localparam int K = E1;
  initial #1 $display("ecul E1=%0d K=%0d", E1, K);
  initial #100 $finish;
endmodule
