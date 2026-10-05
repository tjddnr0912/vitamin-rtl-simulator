module top;
  generate
    case (4'b1x00)
      4'b1x00: begin : g_a initial $display("F06 a"); end
      default: begin : g_def initial $display("F06 def"); end
    endcase
  endgenerate
endmodule
