`timescale 1ns/1ns
module t;
  if (1) begin : b
    localparam [64:0] L = {64'd0, ((4'd15 + 4'd1) inside {5'b0?000})};
    case (1) L: begin : g initial #1 $display("BG_N item"); end default: begin : h initial #1 $display("BG_N default"); end endcase
  end
endmodule
