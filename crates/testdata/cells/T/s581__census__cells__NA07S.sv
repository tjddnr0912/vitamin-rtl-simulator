module top;
  if (1) begin : b1
    localparam logic signed [3:0] K = -1;
    case (K)
      4'b1111: begin : g_b1hit wire [7:0] w = 8'd1; initial #1 $display("NA07S b1hit %0d", w); end
      default: begin : g_b1def wire [7:0] w = 8'd2; initial #1 $display("NA07S b1def %0d", w); end
    endcase
  end
  if (1) begin : b2
    localparam logic [3:0] K = 4'd15;
    case (K)
      -1: begin : g_b2hit wire [7:0] w = 8'd3; initial #1 $display("NA07S b2hit %0d", w); end
      default: begin : g_b2def wire [7:0] w = 8'd4; initial #1 $display("NA07S b2def %0d", w); end
    endcase
  end
endmodule
