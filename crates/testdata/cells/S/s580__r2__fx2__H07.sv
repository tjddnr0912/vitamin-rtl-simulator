`timescale 1ns/1ns
module t;
  logic signed [3:0] sa, s4; logic signed [7:0] s8, s8n, s8b; logic signed [4:0] s5; logic signed [67:0] s68;
  logic signed [63:0] s64; logic [3:0] u4;
  localparam signed [3:0] SA = -4'sd2, CS4 = 4'sb1000;
  localparam signed [7:0] S8 = 8'sd0, S8N = -8'sd4, CS8B = 8'sb1111_1000;
  localparam signed [63:0] S64 = -64'sd2;
  localparam C1 = (SA >>> 1) inside {4'sb111?};
  localparam C2 = (S64 >>> 1) inside {64'shFFFF_FFFF_FFFF_FFF?};
  localparam C3 = (SA + S8) inside {8'sb1111_111?};
  localparam C4 = (S8N / 8'sd2) inside {8'sb1111_111?};
  localparam C5 = (S8N % 8'sd3) inside {8'sb1111_111?};
  localparam C6 = (CS4 + S8) ==? 4'sb1?00;
  localparam C7 = (CS8B >>> 1) ==? 8'sb1111_1?00;
  localparam C8 = (SA >>> 1) !=? 4'sb111?;
  localparam C9 = (SA >>> 1) inside {4'b011?};
  initial begin
    sa = -2; s4 = 4'sb1000; s8 = 0; s8n = -4; s8b = 8'sb1111_1000; s5 = -2; s68 = 0; s64 = -2; u4 = 4'b1110;
    #1;
    $display("R %b %b %b %b %b %b %b %b %b", (sa >>> 1) inside {4'sb111?}, (s64 >>> 1) inside {64'shFFFF_FFFF_FFFF_FFF?},
             (sa + s8) inside {8'sb1111_111?}, (s8n / 8'sd2) inside {8'sb1111_111?}, (s8n % 8'sd3) inside {8'sb1111_111?},
             (s4 + s8) ==? 4'sb1?00, (s8b >>> 1) ==? 8'sb1111_1?00, (sa >>> 1) !=? 4'sb111?, (sa >>> 1) inside {4'b011?});
    $display("C %b %b %b %b %b %b %b %b %b", C1, C2, C3, C4, C5, C6, C7, C8, C9);
    $display("more %b %b %b %b", (s5 >>> 1) inside {4'sb111?}, (s4 + s68) ==? 4'sb1?00, (u4 >>> 1) inside {4'sb011?}, (s4 + s8) inside {4'sb1?00});
    $finish;
  end
endmodule
