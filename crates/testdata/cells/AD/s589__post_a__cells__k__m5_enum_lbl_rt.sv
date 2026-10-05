package q;
  localparam int W = 3;
  function automatic int h(input int x);
    typedef enum {A, B, W} e_t;
    logic [W:0] t;
    t = x;
    return t;
  endfunction
endpackage
module top;
  localparam int W = 7;
  int v;
  initial begin v = q::h(1000); #1 $display("v=%0d", v); $finish; end
  initial #50 $finish;
endmodule
