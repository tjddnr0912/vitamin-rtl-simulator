module top;
  generate
    case (32'hFFFFFFFF)
      -1: begin : g_a initial $display("S16 a"); end
      default: begin : g_def initial $display("S16 def"); end
    endcase
  endgenerate
endmodule
