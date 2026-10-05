module top;
  localparam logic [31:0] G = 32'd7;
  for (genvar G = -1; G < 0; G++) begin : g
    case (-64'sd1)
      G: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("B6 a %m %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("B6 def %m %0d", w); end
    endcase
  end
endmodule
