`timescale 1ns/1ns
module t;
  localparam logic [127:0] P = 128'h1_0000_0000_0000_000C;
  localparam L = (P inside {4'b1?00});
  logic [(P inside {4'b1?00}) : 0] rb;
  if (P inside {4'b1?00}) begin : gi initial #1 $display("W128nin gif then"); end else begin : ge initial #1 $display("W128nin gif else"); end
  case (1) (P inside {4'b1?00}): begin : gc initial #1 $display("W128nin gcase item"); end default: begin : gd initial #1 $display("W128nin gcase default"); end endcase
  initial #2 $display("W128nin L=%b bits=%0d", L, $bits(rb));
endmodule
