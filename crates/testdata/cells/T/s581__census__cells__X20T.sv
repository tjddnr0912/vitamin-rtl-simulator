module top;
  generate
    case (0)
      1'bx ? 1 : 1: begin : g_a initial $display("X20T a"); end
      default: begin : g_def initial $display("X20T def"); end
    endcase
  endgenerate
endmodule
