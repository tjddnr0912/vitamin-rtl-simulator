package pk; localparam logic [7:0] K = 8'd5; endpackage
module top;
  generate
    case (5)
      pk::K: begin : g_a initial $display("L16 a"); end
      default: begin : g_def initial $display("L16 def"); end
    endcase
  endgenerate
endmodule
