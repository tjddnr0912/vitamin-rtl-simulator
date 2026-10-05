`timescale 1ns/1ns
module t;
  localparam logic signed [63:0] C64 = -64'sd4;
  localparam logic [63:0] U64 = 64'hFFFF_FFFF_FFFF_FFFC;
  localparam logic signed [3:0] CS4 = 4'sb1000;
  localparam logic [67:0] U68 = {4'hF, 64'h1};
  localparam logic signed [67:0] S68 = -68'sd4;
  localparam bit E1 = C64 ==? 4'sb1?00;
  localparam bit E2 = C64 ==? 4'sb?100;
  localparam bit E3 = U64 ==? 4'b1?00;
  localparam bit E4 = U64 ==? 64'hFFFF_FFFF_FFFF_FFF?;
  localparam bit E5 = U64 ==? 'bx0;
  localparam bit E6 = U64 ==? 'b1x0;
  localparam bit E7 = CS4 ==? 64'shFFFF_FFFF_FFFF_FF?8;
  localparam bit E8 = CS4 ==? 64'hFFFF_FFFF_FFFF_FF?8;
  localparam bit E9 = U64 ==? 'x;
  localparam bit E10 = U64 ==? '1;
  localparam bit E11 = C64 !=? 64'shFFFF_FFFF_FFFF_FFF?;
  localparam bit W1 = U68 ==? 'bx1;
  localparam bit W2 = U68 ==? 32'bx1;
  localparam bit W3 = S68 ==? 4'sb1?00;
  localparam bit W4 = S68 ==? 4'sb?100;
  localparam bit W5 = U68 ==? 'x;
  localparam bit W6 = U68 ==? '1;
  localparam bit W7 = 68'bx100 ==? 68'b?100;
  localparam bit W8 = (U68 + 68'd0) ==? 4'b0001;
  localparam bit X1 = 4'bx100 ==? 4'b?100;
  localparam bit X2 = 4'b1x00 ==? 4'b1?00;
  localparam bit X3 = 4'bx100 ==? 4'b1?01;
`ifndef NO_INSIDE
  localparam bit N1 = U64 inside {'bx0};
  localparam bit N2 = S68 inside {4'sb1?00};
  localparam bit N3 = C64 inside {4'sb?100, 4'b0000};
`endif
  logic signed [63:0] c64 = -64'sd4; logic [63:0] u64 = 64'hFFFF_FFFF_FFFF_FFFC; logic signed [3:0] cs4 = 4'sb1000;
  logic [67:0] u68 = {4'hF, 64'h1}; logic signed [67:0] s68 = -68'sd4;
  initial begin
    #1;
    $display("RE %b %b %b %b %b %b %b %b %b %b %b", c64 ==? 4'sb1?00, c64 ==? 4'sb?100, u64 ==? 4'b1?00, u64 ==? 64'hFFFF_FFFF_FFFF_FFF?, u64 ==? 'bx0, u64 ==? 'b1x0, cs4 ==? 64'shFFFF_FFFF_FFFF_FF?8, cs4 ==? 64'hFFFF_FFFF_FFFF_FF?8, u64 ==? 'x, u64 ==? '1, c64 !=? 64'shFFFF_FFFF_FFFF_FFF?);
    $display("KE %b %b %b %b %b %b %b %b %b %b %b", E1, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11);
    $display("RW %b %b %b %b %b %b %b %b", u68 ==? 'bx1, u68 ==? 32'bx1, s68 ==? 4'sb1?00, s68 ==? 4'sb?100, u68 ==? 'x, u68 ==? '1, 68'bx100 ==? 68'b?100, (u68 + 68'd0) ==? 4'b0001);
    $display("KW %b %b %b %b %b %b %b %b", W1, W2, W3, W4, W5, W6, W7, W8);
    $display("X %b %b %b / %b %b %b", X1, X2, X3, 4'bx100 ==? 4'b?100, 4'b1x00 ==? 4'b1?00, 4'bx100 ==? 4'b1?01);
`ifndef NO_INSIDE
    $display("N %b %b %b / %b %b %b", N1, N2, N3, u64 inside {'bx0}, s68 inside {4'sb1?00}, c64 inside {4'sb?100, 4'b0000});
`endif
    #1 $finish;
  end
endmodule
