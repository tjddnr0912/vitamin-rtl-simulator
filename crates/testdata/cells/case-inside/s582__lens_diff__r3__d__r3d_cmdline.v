`timescale 1ns/1ns
module top;
  reg [3:0] `NM;
  reg [3:0] x, m;
  initial begin `NM = 4'b0110; x = 4'd3; case (x) inside[2:1]: m = 1; default: m = 0; endcase $display("cmdline m=%0d", m); $finish; end
  initial #1000 $finish;
endmodule
