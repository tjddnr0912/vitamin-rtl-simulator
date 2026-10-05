module top;
  logic a = 0, b = 0; logic [1:0] y;
  function automatic void fv(input logic x, input logic z);
    unique if (x) y = 1; else if (z) y = 2;
  endfunction
  initial begin #1 fv(a, b); $display("t=%0t fvoid done", $time); #1 $finish; end
  initial #100 $finish;
endmodule
