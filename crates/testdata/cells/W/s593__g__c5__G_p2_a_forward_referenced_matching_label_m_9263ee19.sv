`timescale 1ns/1ns
module t;
  if (1) begin : gb
    case (8'd99)
      8'd1: begin : g wire [7:0] w = 8'd1; initial #1 $display("D1P one %0d", w); end
      K: begin : g wire [7:0] w = 8'd200; initial #1 $display("D1P k %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("D1P def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam logic [7:0] K = 8'd99;
  end
endmodule
