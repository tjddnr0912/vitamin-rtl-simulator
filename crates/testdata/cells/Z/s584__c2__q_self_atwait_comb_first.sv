module top;
  logic a, b; int n = 0;
  always_comb b = a;
  always begin @(b) begin n++; $display("GOT t=%0t b=%b", $time, b); end end
  initial a = 1'b1;
  initial begin #1 $display("t=%0t n=%0d b=%b", $time, n, b); $finish; end
  initial #100 $finish;
endmodule
