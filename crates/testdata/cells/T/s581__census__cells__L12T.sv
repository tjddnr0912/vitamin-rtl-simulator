module top;
  generate
    case (2)
      1.0: begin : g_a initial $display("L12T a"); end
      default: begin : g_def initial $display("L12T def"); end
    endcase
  endgenerate
endmodule
