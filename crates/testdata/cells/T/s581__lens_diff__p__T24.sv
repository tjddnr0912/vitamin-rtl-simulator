`timescale 1ns/1ns
module top;
  localparam [64:0] LP = {64'd0, 1'b1};
  logic [5:0] before6;
  case (1)
    LP: begin : g logic [7:0] w; initial #1 $display("@A %0d %h", $bits(w), w); end
    default: begin : g logic [3:0] w; initial #1 $display("@D %0d %h", $bits(w), w); end
  endcase
  assign g.w = 8'hA5;
  initial #2 $display("@top %0d %h", $bits(g.w), g.w);
  case (6)
    $bits(before6): begin : h logic [2:0] q = 3'd5; end
    default: begin : h logic [1:0] q = 2'd1; end
  endcase
  initial #3 $display("@h %0d %0d", $bits(h.q), h.q);
  case (7)
    $bits(after7): begin : k logic [2:0] q = 3'd6; end
    default: begin : k logic [1:0] q = 2'd2; end
  endcase
  initial #4 $display("@k %0d %0d", $bits(k.q), k.q);
  logic [6:0] after7;
endmodule
