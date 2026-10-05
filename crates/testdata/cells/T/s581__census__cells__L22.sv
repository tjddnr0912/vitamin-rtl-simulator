module top;
  localparam logic signed [3:0] PS = -1;
  generate
    case (-1)
      PS: begin : g_a initial $display("L22 a"); end
      default: begin : g_def initial $display("L22 def"); end
    endcase
  endgenerate
endmodule
