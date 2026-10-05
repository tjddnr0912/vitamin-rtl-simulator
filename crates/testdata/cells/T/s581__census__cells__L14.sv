module top;
  localparam P = 3;
  generate
    case (3)
      top.P: begin : g_a initial $display("L14 a"); end
      default: begin : g_def initial $display("L14 def"); end
    endcase
  endgenerate
endmodule
