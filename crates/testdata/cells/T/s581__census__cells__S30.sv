module top;
  localparam UP = 4'sb1111;
  generate
    case (UP)
      4'b1111: begin : g_a initial $display("S30 a"); end
      default: begin : g_def initial $display("S30 def"); end
    endcase
  endgenerate
endmodule
