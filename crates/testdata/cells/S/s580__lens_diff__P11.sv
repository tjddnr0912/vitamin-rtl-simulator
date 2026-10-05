`timescale 1ns/1ns
module t;
  logic [3:0] v, a4; logic [7:0] w8, r1, r2, r3, r4, r5; logic c;
  wire [7:0] cw = c ? (v inside {4'b1?00}) : 8'hFF;
  wire [7:0] sw = 8'(a4 + (v inside {4'b1?00}));
  function automatic logic [3:0] inc(inout logic [3:0] k); k = k + 1; return k; endfunction
  logic [3:0] kk;
  initial #1000 $finish;
  initial begin
    v = 4'b1100; a4 = 4'd15; c = 1; w8 = 8'b1010_0110; kk = 4'b1011;
    #1;
    r1 = 8'(v inside {4'b1?00});
    r2 = 8'(a4 + (v inside {4'b1?00}));
    r3 = {8{v inside {4'b1?00}}};
    r4 = $signed(v inside {4'b1?00});
    r5 = w8[(v inside {4'b1?00}) * 4 +: 4];
    $display("S01 %b", r1); $display("S02 %b", r2); $display("S03 %b", r3); $display("S04 %b", r4); $display("S05 %b", r5);
    $display("S06 %b", cw); $display("S07 %b", sw);
    $display("S08 %b", (v inside {4'b1?00}) ==? 1'b?);
    $display("S09 %b", (v inside {4'b1?00}) && (v inside {4'b?100}));
    $display("S10 %b", inc(kk) inside {4'b11?0});
    $display("S11 %b", kk);
    $display("S12 %0d", (v inside {4'b1?00}) + (v inside {4'b0?00}) + (v inside {4'b11??}));
  end
endmodule
