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
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
