`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  logic [(U ==? 'b1?00)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
