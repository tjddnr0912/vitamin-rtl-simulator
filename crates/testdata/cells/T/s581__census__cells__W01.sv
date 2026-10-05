module top;
  generate
    case (4'd15 + 4'd1)
      0: begin : g_a initial $display("W01 a"); end
      16: begin : g_b initial $display("W01 b"); end
      default: begin : g_def initial $display("W01 def"); end
    endcase
  endgenerate
endmodule
