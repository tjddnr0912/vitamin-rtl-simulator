module top;
  generate
    case (8'd200 + 8'd100)
      9'd300: begin : g_a initial $display("W20 a"); end
      8'd44: begin : g_b initial $display("W20 b"); end
      default: begin : g_def initial $display("W20 def"); end
    endcase
  endgenerate
endmodule
