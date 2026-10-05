module top;
  localparam int PI = -1;
  generate
    case (32'hFFFFFFFF)
      PI: begin : g_a initial $display("S39 a"); end
      default: begin : g_def initial $display("S39 def"); end
    endcase
  endgenerate
endmodule
