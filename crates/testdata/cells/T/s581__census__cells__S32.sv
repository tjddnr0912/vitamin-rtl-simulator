module top;
  localparam logic signed [7:0] PS8 = -1;
  generate
    case (PS8)
      8'hFF: begin : g_a initial $display("S32 a"); end
      default: begin : g_def initial $display("S32 def"); end
    endcase
  endgenerate
endmodule
