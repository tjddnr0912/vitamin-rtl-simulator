module top;
  logic a, b; logic [1:0] w, r, q;
  function logic [1:0] f(input logic x, input logic z); return {x, z}; endfunction
  assign w = f(a, b);
  always_comb begin r = 0; unique if (w[1]) r = 1; else if (w[0]) r = 2; end
  always_latch begin q = 0; unique if (a) q = 1; else if (b) q = 2; end
  initial begin a = 0; b = 1; #1 $display("t=%0t r=%0d q=%0d", $time, r, q); #1 b = 0; #1 $display("t=%0t r=%0d q=%0d", $time, r, q); #1 $finish; end
endmodule
