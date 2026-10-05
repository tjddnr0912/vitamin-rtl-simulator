module top;
  if (1) begin : gb
    case (-1)
      32'hFFFFFFFF: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("D3 a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("D3 def %0d", w); end
    endcase
  end
endmodule
