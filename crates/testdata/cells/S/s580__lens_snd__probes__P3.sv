// P3: S3 sign source per left-operand kind (shared builder; `==?` twin runs on iverilog natively), S6 discriminating cell, S2 x/z override
class K;
  function logic signed [3:0] m(); return -4'sd4; endfunction
endclass
module sub2 #(parameter logic [3:0] P = 4'b0000) (input logic [3:0] v);
  initial #1 $display("OVX P=%b eq=%b wq=%b", P, v == P, v ==? P);
`ifndef NO_INSIDE
  initial #1 $display("OVX in=%b", v inside {P});
`endif
endmodule
module t;
  function automatic logic signed [3:0] f(); return -4'sd4; endfunction
  logic signed [3:0] s4a, s4b, s4p;
  logic signed [3:0] sarr [0:3];
  int i32;
  logic c;
  logic [3:0] a, b;
  K k;
  sub2 #(.P(4'b1x00)) u_x(.v(4'b1100));
  initial begin
    k = new; s4a = -4; s4b = 3; s4p = 4; c = 1; i32 = -4; sarr[1] = -4; a = 1; b = 4'b1000;
    $display("W1=%b W2=%b W3=%b W4=%b W5=%b W6=%b W7=%b W8=%b W9=%b", f() ==? 8'sb11111?00, i32 ==? 64'sb111111111111111111111111111111111111111111111111111111111111?100, i32 ==? 65'sb1111111111111111111111111111111111111111111111111111111111111?100,
      (c ? s4a : s4b) ==? 8'sb11111?00, (-s4p) ==? 8'sb11111?00, sarr[1] ==? 8'sb11111?00, k.m() ==? 8'sb11111?00,
      s4a ==? 8'b00001?00, s4a ==? 8'sb00001?00);
    $display("SC1=%b", 8'(a + (b ==? 4'b1x0x)));
`ifndef NO_INSIDE
    $display("I1=%b I2=%b I3=%b I4=%b I5=%b I6=%b I7=%b I8=%b I9=%b", f() inside {8'sb11111?00}, i32 inside {64'sb111111111111111111111111111111111111111111111111111111111111?100}, i32 inside {65'sb1111111111111111111111111111111111111111111111111111111111111?100},
      (c ? s4a : s4b) inside {8'sb11111?00}, (-s4p) inside {8'sb11111?00}, sarr[1] inside {8'sb11111?00}, k.m() inside {8'sb11111?00},
      s4a inside {8'b00001?00}, s4a inside {8'sb00001?00});
    $display("SC3=%b", 8'(a + (b inside {4'b1x0x})));
`endif
    #2 $finish;
  end
endmodule
