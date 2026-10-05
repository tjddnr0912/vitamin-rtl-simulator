module top;
  generate
    case (4'b0000)
      '0: begin : g_a initial $display("F03 a"); end
      default: begin : g_def initial $display("F03 def"); end
    endcase
  endgenerate
endmodule
