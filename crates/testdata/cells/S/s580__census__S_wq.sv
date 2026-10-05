`timescale 1ns/1ns
module t;
  logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4; integer i32;
  initial begin
    s8=-8'sd4; $display("S01q %b", s8 ==? 4'sb1?00);
    s8=-8'sd4; $display("S02q %b", s8 ==? 4'sb?100);
    s8=8'sb01010100; $display("S03q %b", s8 ==? 4'sb?100);
    s8=-8'sd4; $display("S04q %b", s8 ==? 4'b1?00);
    s8=-8'sd4; $display("S05q %b", s8 ==? 8'sb11111?00);
    u8=8'b11111100; $display("S06q %b", u8 ==? 4'sb1?00);
    s8=-8'sd4; $display("S08q %b", s8 ==? 4'sb1100);
    s4=-4'sd4; $display("S09q %b", s4 ==? 8'sb11111?00);
    s4=-4'sd4; $display("S10q %b", s4 ==? 8'b11111?00);
    s4=-4'sd4; $display("S11q %b", s4 ==? 8'sb00001?00);
    s8=8'sb00001100; $display("S12q %b", s8 ==? 4'sb1?00);
    s8=8'sb00001100; $display("S13q %b", s8 ==? 4'b1?00);
    i32=-4; $display("S14q %b", i32 ==? 4'sb1?00);
    i32=-4; $display("S15q %b", i32 ==? 32'sb1111111111111111111111111111?100);
    s8=-8'sd4; $display("S16q %b", s8 ==? 4'sbx100);
    s8=8'sb01010100; $display("S17q %b", s8 ==? 4'sbz100);
    s4=-4'sd4; $display("S19q %b", s4 ==? 8'sb?1111100);
    s4=4'sb0100; $display("S20q %b", s4 ==? 8'sb?0000100);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
