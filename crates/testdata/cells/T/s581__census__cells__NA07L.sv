module top;
  if (1) begin : b1
    localparam [64:0] K = 65'd1;
    case (1)
      K: begin : g_b1a wire [7:0] w = 8'd1; initial #1 $display("NA07L b1a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA07L def %0d", w); end
    endcase
  end
  if (1) begin : b2
    localparam [64:0] K = 65'd2;
    case (1)
      K: begin : g_b2a wire [7:0] w = 8'd2; initial #1 $display("NA07L b2a %0d", w); end
      default: begin : g_b2def wire [7:0] w = 8'd3; initial #1 $display("NA07L b2def %0d", w); end
    endcase
  end
endmodule
