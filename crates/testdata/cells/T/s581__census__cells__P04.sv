package pk; localparam [64:0] KW = {1'b1, 64'd0}; endpackage

module top;
  generate
    case (1)
      pk::KW >> 64: begin : g_a initial $display("P04 a"); end
      default: begin : g_def initial $display("P04 def"); end
    endcase
  endgenerate
endmodule
