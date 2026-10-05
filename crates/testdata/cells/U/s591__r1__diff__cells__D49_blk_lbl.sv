package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  initial begin : b
    typedef enum {E0, E1} e_t;
    #1 $display("d49 b E1=%0d", E1);
  end
  initial #2 $display("d49 top E1=%0d", E1);
  initial #100 $finish;
endmodule
