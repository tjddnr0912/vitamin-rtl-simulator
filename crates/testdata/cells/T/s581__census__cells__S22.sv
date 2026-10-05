module top;
  generate
    case (4'sb1000)
      8: begin : g_a initial $display("S22 a"); end
      default: begin : g_def initial $display("S22 def"); end
    endcase
  endgenerate
endmodule
