module top;
  localparam [64:0] K = 65'd5;
  if (1) begin : gb
    case (7)
      K: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA03F a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA03F def %0d", w); end
    endcase
    localparam logic [3:0] K = 4'd7;
  end
endmodule
