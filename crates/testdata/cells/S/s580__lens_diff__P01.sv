`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define NIN(a,b) ((a) !=? b)
`else
 `define IN(a,b)  ((a) inside {b})
 `define NIN(a,b) (!((a) inside {b}))
`endif
module t;
  logic signed [3:0] s4, s4p, s4x, s4x2;
  logic [3:0] u4;
  logic signed [7:0] s8, s8m, s8n;
  logic [7:0] u8m, u8n;
  bit signed [3:0] bs4;
  integer ig; int i32, i32w; byte by; shortint sh; longint lg; int unsigned iu; byte unsigned bu;
  logic c;
  wire signed [3:0] ws4 = s4;
  typedef struct packed { logic signed [3:0] f; logic [3:0] g; } st_t;
  st_t st;
  typedef struct packed { logic signed [7:0] f8; logic [7:0] g8; } st8_t;
  st8_t st8;
  logic signed [3:0] arr [0:1];
  logic signed [7:0] arr8 [0:1];
  typedef enum int { NEG = -4, POS = 1 } e_t;
  e_t e;
  function automatic logic signed [3:0] fs(input logic signed [3:0] x); return x; endfunction
  function automatic logic [3:0] fu(input logic [3:0] x); return x; endfunction
  function automatic logic signed [7:0] fs8(input logic signed [7:0] x); return x; endfunction
  initial #1000 $finish;
  initial begin
    s4 = -4; s4p = 4; u4 = 4'b1100; s8 = 8'sb0000_1100; c = 1'b1; bs4 = -4;
    ig = -4; i32 = -4; by = -4; sh = -4; lg = -4; iu = -4; bu = 8'hFC;
    st = 8'b1100_1100; arr[1] = -4; arr[0] = 0; e = NEG; s8m = -4; u8m = 8'hFC;
    s8n = 8'sb0101_0100; u8n = 8'b0101_0100; st8 = 16'b0101_0100_0000_0000; arr8[0] = 8'sb0101_0100; arr8[1] = 0; i32w = 32'h54;
    s4x = 4'sbx100; s4x2 = 4'sb1x00;
    #1;
    $display("N01 %b", `IN(s4, 8'sb1111?100));
    $display("N02 %b", `IN(u4, 8'sb1111?100));
    $display("N03 %b", `IN(ws4, 8'sb1111?100));
    $display("N04 %b", `IN(s8[3:0], 8'sb1111?100));
    $display("N05 %b", `IN({s4}, 8'sb1111?100));
    $display("N06 %b", `IN($signed(u4), 8'sb1111?100));
    $display("N07 %b", `IN($unsigned(s4), 8'sb1111?100));
    $display("N08 %b", `IN(c ? s4 : s4p, 8'sb1111?100));
    $display("N09 %b", `IN(c ? s4 : u4, 8'sb1111?100));
    $display("N10 %b", `IN(fs(s4), 8'sb1111?100));
    $display("N11 %b", `IN(fu(u4), 8'sb1111?100));
    $display("N12 %b", `IN(st.f, 8'sb1111?100));
    $display("N13 %b", `IN(arr[1], 8'sb1111?100));
    $display("N14 %b", `IN(bs4, 8'sb1111?100));
    $display("N15 %b", `IN(-s4p, 8'sb1111?100));
    $display("N16 %b", `IN(s4 + 4'sd0, 8'sb1111?100));
    $display("N17 %b", `IN(s4 + 4'd0, 8'sb1111?100));
    $display("N18 %b", `IN(s4 >>> 0, 8'sb1111?100));
    $display("N19 %b", `IN(s4 & 4'sb1111, 8'sb1111?100));
    $display("N20 %b", `IN(s4 & 4'b1111, 8'sb1111?100));
    $display("N22 %b", `IN(4'(s4), 8'sb1111?100));
    $display("N23 %b", `IN(signed'(u4), 8'sb1111?100));
    $display("N24 %b", `IN(unsigned'(s4), 8'sb1111?100));
    $display("K01 %b", `IN(by, 4'sb1?00));
    $display("K02 %b", `IN(sh, 4'sb1?00));
    $display("K03 %b", `IN(ig, 4'sb1?00));
    $display("K04 %b", `IN(i32, 4'sb1?00));
    $display("K05 %b", `IN(lg, 4'sb1?00));
    $display("K06 %b", `IN(iu, 4'sb1?00));
    $display("K07 %b", `IN(bu, 4'sb1?00));
    $display("K08 %b", `IN(e, 4'sb1?00));
    $display("K09 %b", `IN(s8m, 4'sb1?00));
    $display("K10 %b", `IN(s8m[7:0], 4'sb1?00));
    $display("K11 %b", `IN({s8m}, 4'sb1?00));
    $display("K12 %b", `IN(c ? s8m : u8m, 4'sb1?00));
    $display("K13 %b", `IN(fs8(s8m), 4'sb1?00));
    $display("W01 %b", `IN(s8n, 4'sb?100));
    $display("W02 %b", `IN(u8n, 4'sb?100));
    $display("W03 %b", `IN(s8n[7:0], 4'sb?100));
    $display("W04 %b", `IN({s8n}, 4'sb?100));
    $display("W05 %b", `IN($signed(u8n), 4'sb?100));
    $display("W06 %b", `IN(fs8(s8n), 4'sb?100));
    $display("W07 %b", `IN(st8.f8, 4'sb?100));
    $display("W08 %b", `IN(c ? s8n : u8n, 4'sb?100));
    $display("W09 %b", `IN(arr8[0], 4'sb?100));
    $display("W10 %b", `IN(i32w, 4'sb?100));
    $display("W11 %b", `IN(s8n + 0, 4'sb?100));
    $display("W12 %b", `IN(s8n + 8'd0, 4'sb?100));
    $display("W13 %b", `IN(s8n | 8'sd0, 4'sb?100));
    $display("W14 %b", `IN(s8n << 0, 4'sb?100));
    $display("X01 %b", `NIN(s4, 8'sb1111?100));
    $display("X02 %b", `NIN(s8[3:0], 8'sb1111?100));
    $display("X03 %b", `NIN(s8n, 4'sb?100));
    $display("X04 %b", `NIN(s8n[7:0], 4'sb?100));
    $display("X05 %b", `NIN(by, 4'sb1?00));
    $display("Y01 %b", `IN(s4x, 8'sb1111?100));
    $display("Y02 %b", `IN(s4x2, 8'sb1111?100));
  end
endmodule
