module top;
  generate
    case (1)
      1'bx ? 1 : 1: begin : g_a initial $display("X20 a"); end
      default: begin : g_def initial $display("X20 def"); end
    endcase
  endgenerate
endmodule
