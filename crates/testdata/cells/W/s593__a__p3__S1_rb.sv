`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  logic [(SN ==? 4'b?100)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
