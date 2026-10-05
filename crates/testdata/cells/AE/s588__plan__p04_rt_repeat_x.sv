module top;
  logic [3:0] v; int n;
  initial begin v = 4'bxxx1; n = 0; repeat (v) n = n + 1; $display("n=%0d", n); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
