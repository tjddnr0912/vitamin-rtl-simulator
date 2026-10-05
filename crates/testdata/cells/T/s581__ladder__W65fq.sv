`timescale 1ns/1ns
module t;
  localparam logic [64:0] P = 65'hC;
  localparam L = (P ==? 4'b1?00);
  logic [(P ==? 4'b1?00) : 0] rb;
  if (P ==? 4'b1?00) begin : gi initial #1 $display("W65fq gif then"); end else begin : ge initial #1 $display("W65fq gif else"); end
  case (1) (P ==? 4'b1?00): begin : gc initial #1 $display("W65fq gcase item"); end default: begin : gd initial #1 $display("W65fq gcase default"); end endcase
  initial #2 $display("W65fq L=%b bits=%0d", L, $bits(rb));
endmodule
