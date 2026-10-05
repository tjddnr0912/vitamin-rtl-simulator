package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  if (1) begin : g
    typedef enum {E0, E1} e_t;
    initial #1 $display("d08 g E1=%0d", E1);
  end
  initial #2 $display("d08 top E1=%0d", E1);
  initial #100 $finish;
endmodule
