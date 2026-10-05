package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  typedef enum {E[2]} e_t;
  initial #1 $display("d46 E1=%0d", E1);
  initial #100 $finish;
endmodule
