package pk; localparam logic signed [3:0] KS = -1; endpackage

module top;
  generate
    case (4'b1111)
      pk::KS: begin : g_a initial $display("P01 a"); end
      default: begin : g_def initial $display("P01 def"); end
    endcase
  endgenerate
endmodule
