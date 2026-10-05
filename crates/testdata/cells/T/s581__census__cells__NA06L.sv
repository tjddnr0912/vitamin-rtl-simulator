module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (1)
      i: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA06L a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA06L def %0d", w); end
    endcase
  end
endmodule
