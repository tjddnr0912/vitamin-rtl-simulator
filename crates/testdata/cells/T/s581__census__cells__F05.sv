module top;
  generate
    case (4'b1111)
      '1: begin : g_a initial $display("F05 a"); end
      8'd0: begin : g_b initial $display("F05 b"); end
      default: begin : g_def initial $display("F05 def"); end
    endcase
  endgenerate
endmodule
