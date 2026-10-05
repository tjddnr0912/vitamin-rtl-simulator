package pk; localparam E1 = 7; endpackage
module top;
  typedef enum {E0, E1} e_t;
  if (1) begin : g
    import pk::E1;
    initial #1 $display("d07 g E1=%0d", E1);
  end
  initial #2 $display("d07 top E1=%0d", E1);
  initial #100 $finish;
endmodule
