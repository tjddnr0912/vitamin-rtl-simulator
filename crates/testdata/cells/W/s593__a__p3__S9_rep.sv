`timescale 1ns/1ns
module t;
  localparam int I = 12;
  wire [7:0] r = {((I inside {4'b1?00, 4'b0011})+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
