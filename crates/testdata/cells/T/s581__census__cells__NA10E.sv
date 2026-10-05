package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  if (1) begin : gb
    import pk::K;
    case (7)
      K: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA10E a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA10E def %0d", w); end
    endcase
  end
endmodule
