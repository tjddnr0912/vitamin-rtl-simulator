`timescale 1ns/1ns
module t;
  localparam int I = 12;
  logic [(I inside {4'b1?00, 4'b0011})+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
