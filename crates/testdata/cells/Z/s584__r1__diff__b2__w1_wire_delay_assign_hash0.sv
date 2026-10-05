module top;
  logic [1:0] c = 2'd2;
  logic [1:0] y;
  always_comb y = c;
  wire [1:0] #1 wd = y;
  wire [1:0] w0;
  assign #0 w0 = y;
  always @(w0) $display("W0 t=%0t w0=%b", $time, w0);
  always @(wd) $display("WD t=%0t wd=%b", $time, wd);
  initial begin #0 $display("z t=%0t y=%b w0=%b wd=%b", $time, y, w0, wd); #1 $display("o t=%0t y=%b w0=%b wd=%b", $time, y, w0, wd); end
  initial #5 $finish;
endmodule
