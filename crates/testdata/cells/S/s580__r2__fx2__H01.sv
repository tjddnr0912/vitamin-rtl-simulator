`timescale 1ns/1ns
module m #(parameter W = 0) (); initial #1 $display("override W=%0d", W); endmodule
module t;
  localparam [3:0] A = 4'd15;
  localparam [3:0] U4 = 4'b1100;
  localparam L1 = (4'd15 + 4'd1) inside {5'b1?000};
  localparam L2 = (~4'b0000) inside {5'b1111?};
  localparam L3 = (A + 4'd1) inside {5'b1?000};
  localparam L4 = (4'b1000 << 1) inside {5'b1?000};
  localparam L5 = (4'd15 + 4'd1) inside {5'b0?000};
  localparam L6 = (4'hF << 1) inside {8'b0000_111?};
  localparam L7 = (~4'b0011) inside {8'b1111_11?0};
  localparam L8 = (U4 + 4'd4) inside {8'b0001_0?00};
  localparam L9 = (-(4'd4)) inside {8'b1111_1?00};
  localparam L10 = (4'd4 - 4'd5) inside {8'b1111_111?};
  localparam bit LB = (4'd0 - 4'd1) inside {5'b1111?};
  logic [((4'd15 + 4'd1) inside {5'b1?000}) : 0] rb;
  logic [((4'd15 + 4'd1) inside {8'b0000_?000}) ? 7 : 3 : 0] ab;
  if ((4'd15 + 4'd1) inside {5'b1?000}) begin : g1 initial #1 $display("gen-a then"); end
  else begin : g1e initial #1 $display("gen-a else"); end
  if ((4'd15 + 4'd1) inside {5'b0?000}) begin : g2 initial #1 $display("gen-b then"); end
  else begin : g2e initial #1 $display("gen-b else"); end
  m #(.W(((4'd15 + 4'd1) inside {8'b0000_?000}) ? 8 : 2)) u();
  initial begin
    #2 $display("L %b %b %b %b %b %b %b %b %b %b %b", L1, L2, L3, L4, L5, L6, L7, L8, L9, L10, LB);
    $display("bits %0d %0d", $bits(rb), $bits(ab));
    $finish;
  end
endmodule
