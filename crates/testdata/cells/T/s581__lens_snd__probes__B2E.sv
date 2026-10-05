package pk;
  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;
endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  case (1)
    E1: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("B2E a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("B2E def %0d", w); end
  endcase
endmodule
