module top;
  localparam logic signed [3:0] K = -1;
  if (1) begin : gb
    logic [3:0] K;
    case (K)
      4'b1111: begin : g_hit wire [7:0] w = 8'd1; initial #1 $display("NA09S hit %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA09S def %0d", w); end
    endcase
  end
endmodule
