module top;
  localparam logic signed [3:0] PS = -1;
  generate
    case (4'b1111)
      PS: begin : g_a initial $display("S37 a"); end
      default: begin : g_def initial $display("S37 def"); end
    endcase
  endgenerate
endmodule
