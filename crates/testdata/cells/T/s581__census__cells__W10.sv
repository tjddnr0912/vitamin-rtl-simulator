module top;
  localparam logic [3:0] P4 = 4'd15;
  generate
    case (P4 + 4'd1)
      0: begin : g_a initial $display("W10 a"); end
      16: begin : g_b initial $display("W10 b"); end
      default: begin : g_def initial $display("W10 def"); end
    endcase
  endgenerate
endmodule
