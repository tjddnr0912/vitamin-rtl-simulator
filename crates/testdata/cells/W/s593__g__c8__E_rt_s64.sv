`timescale 1ns/1ns
module sub;
  localparam logic signed [63:0] X = -64'sd4;
  logic r; initial begin r = (X ==? 4'sb1?00); $display("RT=%0d", r); end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
