module top;
  localparam M = 8'h62;
  generate
    case (M)
      "a" + 1: begin : g_a initial $display("R05 a"); end
      default: begin : g_def initial $display("R05 def"); end
    endcase
  endgenerate
endmodule
