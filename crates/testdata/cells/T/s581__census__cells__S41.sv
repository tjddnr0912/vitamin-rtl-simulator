module top;
  localparam integer P = -1;
  generate
    case (P)
      -1: begin : g_a initial $display("S41 a"); end
      default: begin : g_def initial $display("S41 def"); end
    endcase
  endgenerate
endmodule
