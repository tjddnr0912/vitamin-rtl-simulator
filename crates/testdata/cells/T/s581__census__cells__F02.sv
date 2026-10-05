module top;
  generate
    case (8'hFF)
      '1: begin : g_a initial $display("F02 a"); end
      default: begin : g_def initial $display("F02 def"); end
    endcase
  endgenerate
endmodule
