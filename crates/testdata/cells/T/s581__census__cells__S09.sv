module top;
  generate
    case (4'b1111)
      -1: begin : g_a initial $display("S09 a"); end
      default: begin : g_def initial $display("S09 def"); end
    endcase
  endgenerate
endmodule
