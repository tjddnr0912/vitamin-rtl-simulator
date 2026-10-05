`timescale 1ns/1ns
module t;
  localparam logic [67:0] P = 68'hC;
  localparam L = (P inside {4'b1?00});
  logic [(P inside {4'b1?00}) : 0] rb;
  if (P inside {4'b1?00}) begin : gi initial #1 $display("W68fin gif then"); end else begin : ge initial #1 $display("W68fin gif else"); end
  case (1) (P inside {4'b1?00}): begin : gc initial #1 $display("W68fin gcase item"); end default: begin : gd initial #1 $display("W68fin gcase default"); end endcase
  initial #2 $display("W68fin L=%b bits=%0d", L, $bits(rb));
endmodule
