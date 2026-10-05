module top;
  localparam logic [3:0] P4 = 4'b1111;
  generate
    case (15)
      P4: begin : g_a initial $display("L21 a"); end
      default: begin : g_def initial $display("L21 def"); end
    endcase
  endgenerate
endmodule
