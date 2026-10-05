`timescale 1ns/1ns
package pk;
  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;
endpackage
module t;
  import pk::*;
  typedef enum {E0, E1} e_t;
  case (1)
    E1: begin : a wire [7:0] w = 8'd1; initial #1 $display("B2E a %0d", w); end
    default: begin : d wire [7:0] w = 8'd99; initial #1 $display("B2E def %0d", w); end
  endcase
  localparam int K = E1;
  initial #2 $display("B2D E1=%0d K=%0d", E1, K);
endmodule
