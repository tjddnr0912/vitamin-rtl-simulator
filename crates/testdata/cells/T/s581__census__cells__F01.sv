module top;
  generate
    case (4'b1111)
      '1: begin : g_a initial $display("F01 a"); end
      default: begin : g_def initial $display("F01 def"); end
    endcase
  endgenerate
endmodule
