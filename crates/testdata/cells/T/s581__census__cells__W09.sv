module top;
  generate
    case (8'd0)
      4'd15 + 4'd1: begin : g_a initial $display("W09 a"); end
      default: begin : g_def initial $display("W09 def"); end
    endcase
  endgenerate
endmodule
