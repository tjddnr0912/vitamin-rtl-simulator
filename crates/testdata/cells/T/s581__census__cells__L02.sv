module top;
  localparam P = 3;
  generate
    case (3)
      P: begin : g_a initial $display("L02 a"); end
      default: begin : g_def initial $display("L02 def"); end
    endcase
  endgenerate
endmodule
