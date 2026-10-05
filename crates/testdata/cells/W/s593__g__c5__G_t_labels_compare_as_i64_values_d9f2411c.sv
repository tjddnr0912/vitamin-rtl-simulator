`timescale 1ns/1ns
module t;
  localparam P1 = (4'b1100 ==? 4'b1?00);
  localparam [64:0] LPA = {64'd0, P1};
  localparam logic [7:0] P = 8'h62;
  case (1) LPA: begin : i_T1 initial $display("T1 item"); end default: begin : d_T1 initial $display("T1 default"); end endcase
  case (P) "a" + 1: begin : i_L06 initial $display("L06 item"); end default: begin : d_L06 initial $display("L06 default"); end endcase
  case (-1) 32'hFFFFFFFF: begin : i_S06 initial $display("S06 item"); end default: begin : d_S06 initial $display("S06 default"); end endcase
endmodule
