package pk; localparam KU = 4'sb1111; endpackage

module top;
  generate
    case (4'b1111)
      pk::KU: begin : g_a initial $display("P06 a"); end
      default: begin : g_def initial $display("P06 def"); end
    endcase
  endgenerate
endmodule
