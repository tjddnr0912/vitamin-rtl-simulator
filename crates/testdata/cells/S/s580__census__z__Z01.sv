`timescale 1ns/1ns
module t;
  logic [7:0] u8; logic signed [7:0] s8; logic [3:0] v;
  function automatic logic signed [7:0] g(); return -8'sd4; endfunction
  function automatic logic signed [3:0] g4(); return -4'sd4; endfunction
  initial begin
    u8 = 8'b0000_0100; $display("Z01a %b", u8 inside {4'bx100});
    u8 = 8'b1000_0100; $display("Z01b %b", u8 inside {4'bx100});
    $display("Z01c %b", g() inside {4'sb1?00});
    $display("Z01d %b", g4() inside {8'sb1111_1?00});
    $display("Z01e %b", g4() inside {8'sb0000_1?00});
    s8 = -8'sd4; $display("Z01f %b", (s8 + 8'sd0) inside {4'sb1?00});
    s8 = -8'sd4; $display("Z01g %b", s8[3:0] inside {8'sb1111_1?00});
    s8 = -8'sd4; $display("Z01h %b", $signed(s8[3:0]) inside {8'sb1111_1?00});
    v = 4'b1100; $display("Z01i %b", v inside {4'b1?00} && v inside {4'b?100});
    v = 4'b1100; $display("Z01j %b", (v inside {4'b1?00}) ^ 1'b1);
    v = 4'b1100; $display("Z01k %b", {4{v inside {4'b1?00}}});
    v = 4'b1100; $display("Z01l %0d", 8'(v inside {4'b1?00}) + 8'd2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
