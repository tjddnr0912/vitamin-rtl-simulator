module top;
  logic a = 1'b0;
  logic y;
  int n;
  always_comb begin y = a; n++; $display("C t=%0t a=%b", $time, a); end
  initial begin #5 a = 1; #1 $display("n=%0d", n); $finish; end
endmodule
