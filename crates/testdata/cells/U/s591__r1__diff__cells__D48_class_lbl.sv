package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  class C;
    typedef enum {E0, E1} e_t;
    function int get(); return E1; endfunction
  endclass
  initial begin C c; c = new; #1 $display("d48 c=%0d E1=%0d", c.get(), E1); end
  initial #100 $finish;
endmodule
