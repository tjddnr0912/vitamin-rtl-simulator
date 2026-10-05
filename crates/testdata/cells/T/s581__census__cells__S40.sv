module top;
  generate
    case (8'sd127)
      8'sd127: begin : g_a initial $display("S40 a"); end
      default: begin : g_def initial $display("S40 def"); end
    endcase
  endgenerate
endmodule
