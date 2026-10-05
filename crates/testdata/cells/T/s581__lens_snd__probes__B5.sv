package pk;
  localparam int unsigned EM = 5;
endpackage
module top;
  import pk::*;
  typedef enum int {EM = -1, EP = 2} e_t;
  case (-64'sd1)
    EM: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("B5 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("B5 def %0d", w); end
  endcase
endmodule
