module sub;
  logic signed [3:0] s4 = 4'sb1100;
  logic signed [7:0] s8 = 8'sb1000_1100;
endmodule
`ifndef NOC
class C;
  bit signed [3:0] p;
  int ip;
  function new(); p = 4'sb1100; ip = -4; endfunction
endclass
`endif
typedef enum logic signed [3:0] {EA = -4, EB = 3} es_t;
typedef enum logic [3:0] {UA = 12, UB = 3} eu_t;
typedef bit signed [3:0] sb4_t;
typedef struct packed { logic signed [3:0] f; logic [3:0] g; } st_t;
module t;
  localparam logic signed [3:0] PS = -4;
  localparam PU = 4'b1100;
  logic signed [3:0] s4; logic [3:0] u4; logic signed [7:0] s8, z8; logic [7:0] u8, zu8;
  logic signed [3:0] m [0:1];
  bit signed [3:0] b4;
  logic c; int k; integer ig; longint lg;
  es_t es; eu_t eu; st_t st;
`ifndef NOC
  C h; logic signed [3:0] q[$];
`endif
  sub uL();
  function signed [3:0] fs(input x); return 4'sb1100; endfunction
  function [3:0] fu(input x); return 4'b1100; endfunction
  function int fi(input x); return -4; endfunction
  initial #100 $finish;
  initial begin
    s4 = 4'sb1100; u4 = 4'b1100; s8 = 8'sb1000_1100; z8 = 0; u8 = 8'b1000_1100; zu8 = 0;
    m[0] = 4'sb1100; m[1] = 0; b4 = 4'sb1100; c = 1; k = 0; ig = -4; lg = -4;
    es = EA; eu = UA; st = 8'b1100_0101;
`ifndef NOC
    h = new; q.push_back(4'sb1100);
`endif
    #1;
    $display("X01 %b", s4 ==? 8'sb1111_1?00);
    $display("X02 %b", u4 ==? 8'sb1111_1?00);
    $display("X03 %b", {s4} ==? 8'sb1111_1?00);
    $display("X04 %b", {1{s4}} ==? 8'sb1111_1?00);
    $display("X05 %b", s8[3:0] ==? 8'sb1111_1?00);
    $display("X06 %b", s8[3-:4] ==? 8'sb1111_1?00);
    $display("X07 %b", s8[3] ==? 2'sb1?);
    $display("X08 %b", $unsigned(s4) ==? 8'sb1111_1?00);
    $display("X09 %b", $signed(u4) ==? 8'sb1111_1?00);
    $display("X10 %b", (s4 == u4) ==? 2'sb1?);
    $display("X11 %b", (c ? s4 : u4) ==? 8'sb1111_1?00);
    $display("X12 %b", (c ? s4 : b4) ==? 8'sb1111_1?00);
    $display("X13 %b", fs(c) ==? 8'sb1111_1?00);
    $display("X14 %b", fu(c) ==? 8'sb1111_1?00);
    $display("X16 %b", es ==? 8'sb1111_1?00);
    $display("X17 %b", eu ==? 8'sb1111_1?00);
    $display("X18 %b", m[0] ==? 8'sb1111_1?00);
    $display("X19 %b", uL.s4 ==? 8'sb1111_1?00);
    $display("X20 %b", b4 ==? 8'sb1111_1?00);
    $display("X22 %b", byte'(u8) ==? 16'sb1111_1111_1000_1?00);
    $display("X23 %b", 4'(s4) ==? 8'sb1111_1?00);
    $display("X24 %b", (~s4) ==? 8'sb0000_00?1);
    $display("X25 %b", (|s4) ==? 2'sb1?);
    $display("X26 %b", (s4 << 1) ==? 8'sb1111_1?00);
    $display("X29 %b", -4 ==? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("X30 %b", 'hC ==? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("X31 %b", 4'shC ==? 8'sb1111_1?00);
    $display("X32 %b", PS ==? 8'sb1111_1?00);
    $display("X33 %b", PU ==? 8'sb1111_1?00);
    $display("X34 %b", uL.s8[3:0] ==? 8'sb1111_1?00);
    $display("X36 %b", $signed(s8[3:0]) ==? 8'sb1111_1?00);
    $display("X37 %b", m[k] ==? 8'sb1111_1?00);
    $display("X39 %b", st.f ==? 8'sb1111_1?00);
    $display("X40 %b", fi(c) ==? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("X41 %b", ig ==? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("X42 %b", lg ==? 68'shF_FFFF_FFFF_FFFF_FFF?);
    $display("Y01 %b", (s4 + z8) ==? 8'sb1111_1?00);
    $display("Y02 %b", ({s4} + z8) ==? 8'sb1111_1?00);
    $display("Y03 %b", (s8[3:0] + z8) ==? 8'sb1111_1?00);
    $display("Y04 %b", (fs(c) + z8) ==? 8'sb1111_1?00);
    $display("Y06 %b", (es + z8) ==? 8'sb1111_1?00);
    $display("Y07 %b", (m[0] + z8) ==? 8'sb1111_1?00);
    $display("Y08 %b", (uL.s4 + z8) ==? 8'sb1111_1?00);
    $display("Y09 %b", ((c ? s4 : u4) + z8) ==? 8'sb1111_1?00);
    $display("Y10 %b", (s8 >>> 1) ==? 8'sb1100_0?10);
    $display("Y11 %b", ($unsigned(s8) >>> 1) ==? 8'sb0100_0?10);
    $display("Y12 %b", (b4 + z8) ==? 8'sb1111_1?00);
    $display("Y14 %b", (4'(s4) + z8) ==? 8'sb1111_1?00);
    $display("Y15 %b", (s4 + zu8) ==? 8'sb1111_1?00);
    $display("Y16 %b", (s8[7:0] >>> 1) ==? 8'sb0100_0?10);
    $display("Y17 %b", ({s8} >>> 1) ==? 8'sb0100_0?10);
    $display("Y18 %b", ((c ? s8 : u8) >>> 1) ==? 8'sb0100_0?10);
    $display("Y19 %b", (es >>> 1) ==? 4'sb11?0);
    $display("Y21 %b", (uL.s4 >>> 1) ==? 4'sb11?0);
    $display("Y22 %b", (m[0] >>> 1) ==? 4'sb11?0);
    $display("Y23 %b", (fs(c) >>> 1) ==? 4'sb11?0);
    $display("Y24 %b", (fu(c) >>> 1) ==? 4'sb01?0);
    $display("Y25 %b", (eu >>> 1) ==? 4'sb01?0);
    $display("Y27 %b", (byte'(u8) >>> 1) ==? 8'sb1100_0?10);
    $display("Y28 %b", (PS >>> 1) ==? 4'sb11?0);
    $display("Y29 %b", (PU >>> 1) ==? 4'sb01?0);
    $display("Y30 %b", (m[k] >>> 1) ==? 4'sb11?0);
    $display("Y32 %b", (st.f >>> 1) ==? 4'sb11?0);
    $display("Y33 %b", (s4[3:0] >>> 1) ==? 4'sb01?0);
    $display("Y34 %b", ({s4} >>> 1) ==? 4'sb01?0);
    $display("Y35 %b", ((s4 == u4) + z8) ==? 8'sb0000_000?);
`ifndef NOI
    $display("I01 %b", s4 inside {8'sb1111_1?00});
`endif
`ifndef NOI
    $display("I02 %b", {s4} inside {8'sb1111_1?00});
`endif
`ifndef NOI
    $display("I03 %b", (s4 + z8) inside {8'sb1111_1?00});
`endif
`ifndef NOI
    $display("I05 %b", (s8 >>> 1) inside {8'sb1100_0?10});
`endif
`ifndef NOI
    $display("I06 %b", ($unsigned(s8) >>> 1) inside {8'sb0100_0?10});
`endif
`ifndef NOI
    $display("I07 %b", (es >>> 1) inside {4'sb11?0});
`endif
`ifndef NOI
    $display("I08 %b", s8[3] inside {2'sb1?});
`endif
`ifndef NOI
    $display("I09 %b", (s8[7:0] >>> 1) inside {8'sb0100_0?10});
`endif
    $display("N01 %b", (s8 >>> 1) !=? 8'sb1100_0?10);
    $display("N02 %b", {s4} !=? 8'sb1111_1?00);
    $display("C01 %b", (s8[7:0] >>> 1) == 8'sb0100_0110);
    $display("C02 %b", st.f == 8'sb1111_1100);
    $display("C03 %b", (st.f >>> 1) == 4'sb1110);
`ifndef NOC
    $display("X15 %b", h.p ==? 8'sb1111_1?00);
    $display("X28 %b", h.ip ==? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("X38 %b", q[0] ==? 8'sb1111_1?00);
    $display("Y05 %b", (h.p + z8) ==? 8'sb1111_1?00);
    $display("Y20 %b", (h.p >>> 1) ==? 4'sb11?0);
    $display("Y31 %b", (q[0] >>> 1) ==? 4'sb11?0);
`ifndef NOI
    $display("I04 %b", h.p inside {8'sb1111_1?00});
`endif
`endif
  end
endmodule
