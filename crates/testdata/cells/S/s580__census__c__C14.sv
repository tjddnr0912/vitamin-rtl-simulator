`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b1100; case (1'b1) (v inside {4'b1?00}): $display("C14 item"); default: $display("C14 default"); endcase #1 $finish; end
  initial #100 $finish;
endmodule
