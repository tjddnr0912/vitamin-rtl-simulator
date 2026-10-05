`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define NIN(a,b) ((a) !=? b)
`else
 `define IN(a,b)  ((a) inside {b})
 `define NIN(a,b) (!((a) inside {b}))
`endif
module sub; logic signed [3:0] s4h; initial s4h = -4; endmodule
module t;
  logic signed [3:0] s4, s4x; logic [3:0] u4; logic signed [7:0] s8, s8n; logic [7:0] u8n; byte by; logic c, clk;
  typedef struct packed { logic signed [3:0] f; logic [3:0] g; } st_t; st_t st;
  typedef enum int { NEG = -4, POS = 1 } e_t; e_t e;
  integer ia [0:1];
  logic signed [3:0] dq [$];
  logic signed [1:0][3:0] pa;
  function automatic logic signed [3:0] fs(input logic signed [3:0] x); return x; endfunction
  class Cp; logic signed [3:0] p; logic signed [7:0] p8; function new(); p = -4; p8 = 8'sb0101_0100; endfunction endclass
  Cp cp;
  sub u();
  wire a01 = `IN(s4, 8'sb1111?100);
  wire a02 = `IN(s8[3:0], 8'sb1111?100);
  wire a03 = `IN(c ? s4 : u4, 8'sb1111?100);
  wire a04, a05, a06, a07, a08, a09, a10, a11, a13, a14, a15;
  wire [7:0] a12;
  assign a04 = `IN(s8n, 4'sb?100);
  assign a05 = `IN(s8n[7:0], 4'sb?100);
  assign a06 = `IN(c ? s8n : u8n, 4'sb?100);
  assign a07 = `IN(by, 4'sb1?00);
  assign a08 = `IN(fs(s4), 8'sb1111?100);
  assign a09 = `IN(st.f, 8'sb1111?100);
  assign a10 = `IN(u.s4h, 8'sb1111?100);
  assign a11 = `IN(s4x, 8'sb1111?100);
  assign a12 = `IN(s4, 8'sb1111?100);
  assign a13 = `NIN(s8n, 4'sb?100);
  assign a14 = `IN(e, 4'sb1?00);
  assign a15 = `IN(s4 + 4'd0, 8'sb1111?100);
  logic q01, q02, q03; logic [7:0] r8;
  always_ff @(posedge clk) begin q01 <= `IN(s4, 8'sb1111?100); q02 <= `IN(s8n[7:0], 4'sb?100); q03 <= `IN(by, 4'sb1?00); end
  initial #1000 $finish;
  initial begin
    s4 = -4; u4 = 4'b1100; s8 = 8'sb0000_1100; s8n = 8'sb0101_0100; u8n = 8'b0101_0100; by = -4; c = 1'b1; clk = 0;
    st = 8'b1100_1100; e = NEG; s4x = 4'sbx100; ia[0] = -4; ia[1] = 0; dq.push_back(-4); pa = 8'b1100_0000;
    cp = new;
    #1 clk = 1; #1;
    $display("A01 %b", a01); $display("A02 %b", a02); $display("A03 %b", a03); $display("A04 %b", a04);
    $display("A05 %b", a05); $display("A06 %b", a06); $display("A07 %b", a07); $display("A08 %b", a08);
    $display("A09 %b", a09); $display("A10 %b", a10); $display("A11 %b", a11); $display("A12 %b", a12);
    $display("A13 %b", a13); $display("A14 %b", a14); $display("A15 %b", a15);
    $display("Q01 %b", q01); $display("Q02 %b", q02); $display("Q03 %b", q03);
    r8 = `IN(s4, 8'sb1111?100); $display("B01 %b", r8);
    $display("C01 %b", `IN(cp.p, 8'sb1111?100));
    $display("C02 %b", `IN(cp.p8, 4'sb?100));
    $display("C03 %b", `IN(ia[0], 4'sb1?00));
    $display("C04 %b", `IN(dq[0], 8'sb1111?100));
    $display("C05 %b", `IN(pa[1], 8'sb1111?100));
    $display("C06 %b", `IN($signed(s8[3:0]), 8'sb1111?100));
    $display("C07 %b", `IN(int'(s4), 4'sb1?00));
    $display("C08 %b", `IN(u.s4h, 8'sb1111?100));
`ifndef IV
    $display("C09 %b", (12.0 inside {4'b1100}));
    $display("C10 %b", (-4.0 inside {8'sb1111_1100}));
`endif
  end
endmodule
