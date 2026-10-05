`timescale 1ns/1ns
module t;
  logic [3:0] v; logic m; logic [3:0] r;
  wire w1; wire [7:0] w8; wire [3:0] y;
  assign w1 = v inside {4'b1?00};
  assign w8 = v inside {4'b1?00};
  assign y = (v inside {4'b1?00}) ? 4'd1 : 4'd2;
  always_comb m = v inside {4'b1?00};
  always @* r = (v inside {4'b1?00}) ? 4'd7 : 4'd3;
  initial begin
    v = 4'b1100; #1 $display("cont %b %b %h comb %b star %0d", w1, w8, y, m, r);
    v = 4'b0100; #1 $display("cont %b %b %h comb %b star %0d", w1, w8, y, m, r);
    $finish;
  end
endmodule
