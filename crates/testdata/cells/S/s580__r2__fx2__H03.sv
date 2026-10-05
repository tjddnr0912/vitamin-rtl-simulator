`timescale 1ns/1ns
module t;
  localparam [99:0] W = 100'd2;
  localparam L1 = 4'b0100 inside {4'sb1?00};
  localparam L2 = 4'b0100 inside {'b1?00};
  localparam L3 = W inside {4'b000?};
  logic [(4'b1100 inside {4'sb1?00}) : 0] rbs;
  logic [(4'b1100 inside {'b1?00}) : 0] rbu;
  logic [(4'b1100 inside {4'b0000, 4'b1?00}) : 0] rb2;
  logic ad [(4'b1100 inside {4'b1?00}) : 0];
  if (4'b0100 inside {4'sb1?00}) begin : g initial #1 $display("gen then"); end
  else begin : ge initial #1 $display("gen else"); end
  initial begin
    #2 $display("L %b %b %b bits %0d %0d %0d size %0d", L1, L2, L3, $bits(rbs), $bits(rbu), $bits(rb2), $size(ad));
    $finish;
  end
endmodule
