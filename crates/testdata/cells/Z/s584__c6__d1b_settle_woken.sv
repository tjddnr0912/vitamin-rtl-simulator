module top;
  logic a = 1'b0;
  wire w;
  logic y;
  int n;
  assign w = ~a;
  always_comb begin y = w; n++; $display("C t=%0t w=%b", $time, w); end
  initial begin #5 a = 1; #1 $display("n=%0d", n); $finish; end
endmodule
