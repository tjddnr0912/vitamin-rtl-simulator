module top;
  typedef enum logic signed [3:0] {EM = -1, EZ = 0} e_t;
  case (4'b1111)
    EM: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA11M a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA11M def %0d", w); end
  endcase
endmodule
