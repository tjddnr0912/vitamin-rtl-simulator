module top;
  logic s;
  wire w;
  logic y;
  assign #0 w = s;
  always_comb begin y = w; $display("C t=%0t w=%b", $time, w); end
  initial begin s = 1'b1; #0 $display("I1 t=%0t w=%b y=%b", $time, w, y); #1 $display("e y=%b", y); $finish; end
endmodule
