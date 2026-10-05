`timescale 1ns/1ns
module t;
  localparam logic [64:0] P = 65'h1_0000_0000_0000_000C;
  localparam L = (P inside {4'b1?00});
  logic [(P inside {4'b1?00}) : 0] rb;
  if (P inside {4'b1?00}) begin : gi initial #1 $display("W65nin gif then"); end else begin : ge initial #1 $display("W65nin gif else"); end
  case (1) (P inside {4'b1?00}): begin : gc initial #1 $display("W65nin gcase item"); end default: begin : gd initial #1 $display("W65nin gcase default"); end endcase
  initial #2 $display("W65nin L=%b bits=%0d", L, $bits(rb));
endmodule
