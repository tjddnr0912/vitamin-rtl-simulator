module top;
  generate
    case (8'h62)
      "a" + 1: begin : g_a initial $display("L26 a"); end
      default: begin : g_def initial $display("L26 def"); end
    endcase
  endgenerate
endmodule
