module top;
  localparam U8 = 8'hFF;
  generate
    case (U8)
      -1: begin : g_a initial $display("S46 a"); end
      default: begin : g_def initial $display("S46 def"); end
    endcase
  endgenerate
endmodule
