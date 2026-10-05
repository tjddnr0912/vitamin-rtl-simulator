`timescale 1ns/1ns
module t;
  logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4; integer i32;
  initial begin
    s8=-8'sd4; $display("S01 %b", s8 inside {4'sb1?00});
    s8=-8'sd4; $display("S02 %b", s8 inside {4'sb?100});
    s8=8'sb01010100; $display("S03 %b", s8 inside {4'sb?100});
    s8=-8'sd4; $display("S04 %b", s8 inside {4'b1?00});
    s8=-8'sd4; $display("S05 %b", s8 inside {8'sb11111?00});
    u8=8'b11111100; $display("S06 %b", u8 inside {4'sb1?00});
    s8=-8'sd4; $display("S07 %b", s8 inside {-8'sd4});
    s8=-8'sd4; $display("S08 %b", s8 inside {4'sb1100});
    s4=-4'sd4; $display("S09 %b", s4 inside {8'sb11111?00});
    s4=-4'sd4; $display("S10 %b", s4 inside {8'b11111?00});
    s4=-4'sd4; $display("S11 %b", s4 inside {8'sb00001?00});
    s8=8'sb00001100; $display("S12 %b", s8 inside {4'sb1?00});
    s8=8'sb00001100; $display("S13 %b", s8 inside {4'b1?00});
    i32=-4; $display("S14 %b", i32 inside {4'sb1?00});
    i32=-4; $display("S15 %b", i32 inside {32'sb1111111111111111111111111111?100});
    s8=-8'sd4; $display("S16 %b", s8 inside {4'sbx100});
    s8=8'sb01010100; $display("S17 %b", s8 inside {4'sbz100});
    s8=-8'sd4; $display("S18 %b", s8 inside {4'sb1100, 4'sb1?00});
    s4=-4'sd4; $display("S19 %b", s4 inside {8'sb?1111100});
    s4=4'sb0100; $display("S20 %b", s4 inside {8'sb?0000100});
    s8=-8'sd4; $display("S21 %b", s8 inside {[-8'sd5:-8'sd3]});
    s8=-8'sd4; $display("S22 %b", s8 inside {[-8'sd5:-8'sd3], 4'sb0?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
