`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam signed [7:0] S2 = 8'sd12;
  localparam [7:0] U = 8'hF4;
  localparam [3:0] U4 = 4'b1100;
  localparam [3:0] T = 5'b10100;
  localparam signed [3:0] SP = 4'sd4;
  localparam signed [3:0] SN = -4'sd4;
  localparam signed [7:0] SPOS = 8'sd84;
  localparam int PI = 5;
  localparam real R = 12.0;
  localparam L03 = `IN(U, 4'b?100);
  localparam L04 = `IN(U4, 8'b0000_1?00);
  localparam L05 = `IN(T, 4'b?100);
  localparam L06 = `IN(SP, 8'b0000_?100);
  localparam L09 = `IN(4'd15 + 4'd1, 8'b0001_?000);
  localparam L10 = `IN(4'hF << 1, 8'b0001_111?);
  localparam L11 = `IN(4'd15 + 4'd1, 4'b000?);
  localparam L12 = `IN(~4'b0011, 8'b1111_11?0);
  localparam L14 = `IN(U4 + 4'd4, 8'b0001_0?00);
  localparam L15 = `IN(SPOS, 4'b?100);
  localparam L17 = `IN(PI, 64'h0000_0000_0000_000?);
`ifdef B
  localparam L01 = `IN(S, 4'sb?100);
  localparam L02 = `IN(S2, 4'sb1?00);
  localparam L07 = `IN(SN, 4'b1?00);
  localparam L08 = `IN(SN, 8'b0000_1?00);
  localparam L13 = `IN(-4'sd4, 8'sb1111_1?00);
`endif
`ifdef RL
 `ifdef IV
  localparam LR = (R == 4'b1?00);
 `else
  localparam LR = R inside {4'b1?00};
 `endif
`endif
  logic [3:0] a4, b4, c4, u4v;
  logic signed [7:0] vS, vS2;
  logic w09, w14, w12;
  assign w09 = `IN(a4 + b4, 8'b0001_?000);
  assign w14 = `IN(u4v + 4'd4, 8'b0001_0?00);
  assign w12 = `IN(~c4, 8'b1111_11?0);
  if (`IN(4'd15 + 4'd1, 8'b0001_?000)) begin : g9 initial #2 $display("G09 then"); end
  else begin : g9e initial #2 $display("G09 else"); end
  if (`IN(U, 4'b?100)) begin : g3 initial #2 $display("G03 then"); end
  else begin : g3e initial #2 $display("G03 else"); end
`ifdef B
  if (`IN(S, 4'sb?100)) begin : g1 initial #2 $display("G01 then"); end
  else begin : g1e initial #2 $display("G01 else"); end
`endif
  initial #1000 $finish;
  initial begin
    a4 = 15; b4 = 1; c4 = 4'b0011; u4v = 12; vS = 8'sd84; vS2 = 8'sd12;
    #1;
    $display("L03 %b", L03); $display("L04 %b", L04); $display("L05 %b", L05); $display("L06 %b", L06);
    $display("L09 %b", L09); $display("L10 %b", L10); $display("L11 %b", L11); $display("L12 %b", L12);
    $display("L14 %b", L14); $display("L15 %b", L15); $display("L17 %b", L17);
`ifdef B
    $display("L01 %b", L01); $display("L02 %b", L02); $display("L07 %b", L07); $display("L08 %b", L08); $display("L13 %b", L13);
`endif
`ifdef RL
    $display("LR %b", LR);
`endif
    $display("R01 %b", `IN(S, 4'sb?100));
    $display("R02 %b", `IN(S2, 4'sb1?00));
    $display("R01v %b", `IN(vS, 4'sb?100));
    $display("R02v %b", `IN(vS2, 4'sb1?00));
    $display("R03 %b", `IN(U, 4'b?100));
    $display("R07 %b", `IN(SN, 4'b1?00));
    $display("R08 %b", `IN(SN, 8'b0000_1?00));
    $display("R09 %b", `IN(4'd15 + 4'd1, 8'b0001_?000));
    $display("R09v %b", `IN(a4 + b4, 8'b0001_?000));
    $display("R10v %b", `IN(a4 << 1, 8'b0001_111?));
    $display("R12v %b", `IN(~c4, 8'b1111_11?0));
    $display("R14v %b", `IN(u4v + 4'd4, 8'b0001_0?00));
    $display("R13 %b", `IN(-4'sd4, 8'sb1111_1?00));
    #1;
    $display("W09 %b", w09); $display("W14 %b", w14); $display("W12 %b", w12);
  end
endmodule
