`timescale 1ns/1ns
module t;
  logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4; integer i32;
  function automatic logic signed [7:0] g8(); return -8'sd4; endfunction
  function automatic logic signed [3:0] g4(); return -4'sd4; endfunction
  initial begin
    s8 = -8'sd4;      $display("ss-narrow-pat-msb1 %b", s8 inside {4'sb1?00});
    s8 = -8'sd4;      $display("ss-narrow-pat-msbq %b", s8 inside {4'sb?100});
    s8 = 8'sb01010100; $display("ss-narrow-pat-msbq-pos %b", s8 inside {4'sb?100});
    s8 = -8'sd4;      $display("ss-narrow-pat-msbx %b", s8 inside {4'sbx100});
    s8 = 8'sb01010100; $display("ss-narrow-pat-msbz %b", s8 inside {4'sbz100});
    s8 = 8'sb00001100; $display("ss-narrow-pat-ext-ones %b", s8 inside {4'sb1?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-sext %b", s4 inside {8'sb11111?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-zero-pat %b", s4 inside {8'sb00001?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-wild-msb %b", s4 inside {8'sb?1111100});
    i32 = -4;         $display("ss-integer %b", i32 inside {4'sb1?00});
    s8 = -8'sd4;      $display("su-pattern-unsigned %b", s8 inside {4'b1?00});
    s4 = -4'sd4;      $display("su-wide-unsigned-pat %b", s4 inside {8'b11111?00});
    u8 = 8'b11111100; $display("us-lhs-unsigned %b", u8 inside {4'sb1?00});
    s8 = 8'sb00001100; $display("su-zero-ext-hit %b", s8 inside {4'b1?00});
    $display("call-lhs %b", g8() inside {4'sb1?00});
    $display("call-lhs-narrow %b", g4() inside {8'sb1111_1?00});
    s8 = -8'sd4;      $display("select-lhs-unsigned %b", s8[3:0] inside {8'sb1111_1?00});
    s8 = -8'sd4;      $display("signed-select-lhs %b", $signed(s8[3:0]) inside {8'sb1111_1?00});
    $finish;
  end
endmodule
