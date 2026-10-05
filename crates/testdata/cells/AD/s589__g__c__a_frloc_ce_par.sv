package q;
  localparam int W = 3;
  function automatic int h(input int x);
    logic [W:0] t;
    t = 0;
    for (int i = 0; i < x; i++) t = t + 1;
    return t;
  endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
