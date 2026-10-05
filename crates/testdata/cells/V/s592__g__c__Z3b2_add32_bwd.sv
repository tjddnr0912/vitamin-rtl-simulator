module top;
  if (1) begin : gb
    localparam K = 32'hFFFF_FFFF + 1;
    case (K)
      0: begin : g wire [7:0] w = 8'd200; initial #1 $display("@zero %0d bits=%0d", w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
  end
  initial #5 $finish;
endmodule
