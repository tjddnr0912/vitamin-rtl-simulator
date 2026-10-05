module top;
  localparam [64:0] K = 65'd5;
  if (1) begin : gb
    localparam [64:0] K = 65'd7;
    case (K)
      65'd7: begin : g_seven wire [7:0] w = 8'd1; initial #1 $display("NA02S seven %0d", w); end
      65'd5: begin : g_five wire [7:0] w = 8'd2; initial #1 $display("NA02S five %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA02S def %0d", w); end
    endcase
  end
endmodule
