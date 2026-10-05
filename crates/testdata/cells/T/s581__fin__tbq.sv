`timescale 1ns/1ns
module t;
  case (1) (4'b1100 ==? 4'b1?00): begin : i_X11 initial $display("X11 item"); end default: begin : d_X11 initial $display("X11 default"); end endcase
endmodule
