module top;
  localparam logic signed [3:0] PS = -1;
  generate
    case (15)
      PS: begin : g_a initial $display("L23 a"); end
      default: begin : g_def initial $display("L23 def"); end
    endcase
  endgenerate
endmodule
