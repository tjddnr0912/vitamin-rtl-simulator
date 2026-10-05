module top;
  localparam P = 3;
  generate
    case (4)
      P + 1: begin : g_a initial $display("L03 a"); end
      default: begin : g_def initial $display("L03 def"); end
    endcase
  endgenerate
endmodule
