module top;
  logic s;
  wire w;
  logic y;
  int n;
  assign #0 w = s;
  always_comb begin y = w; n++; $display("C t=%0t w=%b", $time, w); end
  initial begin s = 1'b0; #5 s = 1'b1; #1 $display("n=%0d", n); $finish; end
endmodule
