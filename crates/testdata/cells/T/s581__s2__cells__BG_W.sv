`timescale 1ns/1ns
module t;
  if (1) begin : b
    localparam [64:0] L = {64'd0, ((4'd15 + 4'd1) ==? 5'b1?000)};
    case (1) L: begin : g initial #1 $display("BG_W item"); end default: begin : h initial #1 $display("BG_W default"); end endcase
  end
endmodule
