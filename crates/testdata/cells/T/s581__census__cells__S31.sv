module top;
  localparam logic [7:0] PU = 8'hFF;
  generate
    case (PU)
      -1: begin : g_a initial $display("S31 a"); end
      default: begin : g_def initial $display("S31 def"); end
    endcase
  endgenerate
endmodule
