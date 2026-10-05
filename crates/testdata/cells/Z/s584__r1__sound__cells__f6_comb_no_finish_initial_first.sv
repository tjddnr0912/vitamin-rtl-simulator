module top;
  logic a, y;
  initial a = 1;
  always_comb begin y = a; $display("C t=%0t a=%b", $time, a); end
  initial #5 $finish;
endmodule
