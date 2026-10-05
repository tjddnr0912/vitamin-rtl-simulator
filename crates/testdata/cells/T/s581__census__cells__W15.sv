module top;
  generate
    case (~4'd0)
      8'hFF: begin : g_a initial $display("W15 a"); end
      4'hF: begin : g_b initial $display("W15 b"); end
      default: begin : g_def initial $display("W15 def"); end
    endcase
  endgenerate
endmodule
