module top;
  localparam real R = 1.0;
  generate
    case (2)
      R: begin : g_a initial $display("L13T a"); end
      default: begin : g_def initial $display("L13T def"); end
    endcase
  endgenerate
endmodule
