`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  wire [7:0] r = {((U ==? (4'b1?00))+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
