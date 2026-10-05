module top;
  logic a; int n = 0;
  initial a = 1'b1;
  always begin @(a) begin n++; $display("GOT t=%0t a=%b", $time, a); end end
  initial begin #1 $display("t=%0t n=%0d", $time, n); $finish; end
  initial #100 $finish;
endmodule
