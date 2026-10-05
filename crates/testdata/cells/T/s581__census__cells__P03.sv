localparam logic signed [3:0] US = -1;

module top;
  generate
    case (4'b1111)
      US: begin : g_a initial $display("P03 a"); end
      default: begin : g_def initial $display("P03 def"); end
    endcase
  endgenerate
endmodule
