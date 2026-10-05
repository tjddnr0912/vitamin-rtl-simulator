`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define IN2(a,b,c) (((a) ==? b) || ((a) ==? c))
`else
 `define IN(a,b)  ((a) inside {b})
 `define IN2(a,b,c) ((a) inside {b, c})
`endif
`define SH(k, e) $display(`"k %b`", e)
module sub;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
  wire [7:0] wn = 8'hC4;
  logic [7:0] r8 = 8'hC4; logic signed [7:0] s8 = -8'sd60, s8b = -8'sd4; logic signed [3:0] s4 = -4'sd4; logic [3:0] r4 = 4'b1100;
  st_t st = 8'hC4; logic [7:0] arr [0:3] = '{8'h00, 8'hC4, 8'h00, 8'h00};
  logic [7:0] rx = 8'b1x00_0100, rz = 8'bz100_0100; logic [69:0] w70 = 70'h3F_0000_0000_0000_0004;
  parameter P = 4'b1100; parameter signed [3:0] PS = -4'sd4;
  if (1) begin : g logic [7:0] gr = 8'hC4; end
  initial begin
    #2;
    `SH(UP1, `IN(t.top8, 4'b?100));
    `SH(UP2, `IN(t.top8, 8'b1100_0?00));
  end
endmodule
module t;
  logic [7:0] top8 = 8'hC4;
  sub u();
  initial begin
    #1;
    `SH(H01, `IN(t.u.wn, 4'b?100));
    `SH(H02, `IN(t.u.wn, 8'b1100_0?00));
    `SH(H03, `IN(t.u.r8, 4'b?100));
    `SH(H04, `IN(t.u.r4, 8'b0000_1?00));
    `SH(H05, `IN(t.u.s8, 4'sb?100));
    `SH(H06, `IN(t.u.s8, 4'sb01?0));
    `SH(H07, `IN(t.u.s4, 8'sb1111_1?00));
    `SH(H08, `IN(t.u.s4, 8'b1111_1?00));
    `SH(H09, `IN(t.u.r8[5:2], 4'b000?));
    `SH(H10, `IN(t.u.r8[5:2], 8'b0000_000?));
`ifdef STM
    `SH(H11, `IN(t.u.st.a, 4'b1?00));
`endif
`ifdef STM
    `SH(H12, `IN(t.u.st.a, 8'b0000_1?00));
`endif
`ifdef STM
    `SH(H13, `IN(t.u.st.a, 3'b1?0));
`endif
    `SH(H14, `IN(t.u.arr[1], 8'b1100_0?00));
    `SH(H15, `IN(t.u.arr[1], 4'b?100));
    `SH(H16, `IN(t.u.g.gr, 4'b?100));
    `SH(H17, `IN(t.u.g.gr, 8'b11?0_0100));
    `SH(H18, `IN(t.u.P, 4'b1?00));
    `SH(H19, `IN(t.u.P, 8'b0000_1?00));
    `SH(H20, `IN(t.u.PS, 8'sb1111_1?00));
    `SH(H21, `IN(t.u.PS, 8'b1111_1?00));
    `SH(H22, `IN(t.u.rx, 8'b1?00_0100));
    `SH(H23, `IN(t.u.rx, 8'b11?0_010?));
    `SH(H24, `IN(t.u.rx, 8'b11?0_110?));
    `SH(H25, `IN(t.u.rz, 8'b?100_0100));
    `SH(H26, `IN(t.u.rz, 8'b0100_010?));
    `SH(H27, (t.u.r8 !=? 4'b?100));
    `SH(H28, (t.u.rx !=? 8'b11?0_010?));
    `SH(H29, `IN2(t.u.r8, 4'b?100, 8'b1100_0?00));
    `SH(H30, `IN(t.u.w70, 4'b?100));
    `SH(H31, `IN(t.u.w70, 70'h3F_0000_0000_0000_000?));
    `SH(H32, `IN(t.u.r4, 8'sb1111_1?00));
    `SH(H33, `IN(t.u.r8, 4'sb1?00));
    `SH(H34, `IN(t.u.s8b, 4'sb1?00));
    `SH(H35, `IN(t.u.s8b >>> 1, 8'sb1111_111?));
    `SH(H36, `IN(t.u.s4 + 8'sd0, 8'sb1111_1?00));
    `SH(H37, `IN(t.u.s8b >>> 1, 4'sb111?));
    `SH(H38, `IN(t.u.s8b, 8'sb1111_11?0));
    `SH(H39, `IN2(t.u.s4, 8'sb1111_1?00, 4'b0000));
    #3 $finish;
  end
endmodule
