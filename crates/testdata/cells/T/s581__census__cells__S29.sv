module top;
  localparam int PI = -1;
  generate
    case (PI)
      32'hFFFFFFFF: begin : g_a initial $display("S29 a"); end
      default: begin : g_def initial $display("S29 def"); end
    endcase
  endgenerate
endmodule
