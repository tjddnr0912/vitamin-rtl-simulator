module top;
  localparam int PI = -1;
  generate
    case (-1)
      PI: begin : g_a initial $display("S38 a"); end
      default: begin : g_def initial $display("S38 def"); end
    endcase
  endgenerate
endmodule
