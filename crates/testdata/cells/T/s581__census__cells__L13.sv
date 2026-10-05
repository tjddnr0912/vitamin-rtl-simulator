module top;
  localparam real R = 1.0;
  generate
    case (1)
      R: begin : g_a initial $display("L13 a"); end
      default: begin : g_def initial $display("L13 def"); end
    endcase
  endgenerate
endmodule
