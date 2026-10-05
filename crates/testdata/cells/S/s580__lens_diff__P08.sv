`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module late; logic [7:0] u8 = 8'b0101_0100; logic signed [7:0] s8 = 8'sb0101_0100; endmodule
module t;
  logic [64:0] u65; logic [63:0] u64; logic [35:0] v36; logic [7:0] u8n; logic signed [64:0] s65; logic signed [7:0] s8n;
  wire w01 = `IN(u65, 'bx1);
  wire w02 = `IN(u64, 'bz1);
  wire w03 = `IN(v36, 'bx1);
  wire w04 = `IN(u8n, 4'b?100);
  wire w05 = `IN(s65, 'sbx0);
  wire w06 = `IN(u65, 'h?);
  wire w07 = `IN(s8n, 'sb?100);
  wire w08 = `IN(v36, 'hx_0000_0001);
  logic q01, q02, clk;
  always_ff @(posedge clk) begin q01 <= `IN(u65, 'bx1); q02 <= `IN(v36, 'bx1); end
  wire h01 = `IN(uL.u8, 4'sb1?00);
  wire h02 = `IN(uL.s8, 4'sb?100);
  late uL();
  localparam signed [7:0] S = 8'sd84;
  localparam signed [7:0] S2 = 8'sd12;
`ifdef IV
  localparam L01 = (S ==? 4'sb?100);
  localparam L02 = (S2 ==? 4'sb1?00);
  localparam L20 = ((4'd15 + 4'd1) ==? 8'b0000_?000);
  localparam L09 = ((4'd15 + 4'd1) ==? 8'b0001_?000);
`endif
  initial #1000 $finish;
  initial begin
    u65 = {1'b1, 64'h1}; u64 = 64'hF000_0000_0000_0001; v36 = 36'hF_0000_0001; u8n = 8'b0101_0100; s65 = -4; s8n = 8'sb0101_0100; clk = 0;
    #1 clk = 1; #1;
    $display("W01 %b", w01); $display("W02 %b", w02); $display("W03 %b", w03); $display("W04 %b", w04);
    $display("W05 %b", w05); $display("W06 %b", w06); $display("W07 %b", w07); $display("W08 %b", w08);
    $display("Q01 %b", q01); $display("Q02 %b", q02);
    $display("H01 %b", h01); $display("H02 %b", h02);
    $display("H03 %b", `IN(uL.u8, 4'sb1?00));
    $display("H04 %b", `IN(t.u8n, 4'sb?100));
    $display("H05 %b", `IN(t.s8n, 4'sb?100));
`ifdef IV
    $display("L01 %b", L01); $display("L02 %b", L02); $display("L20 %b", L20); $display("L09 %b", L09);
`endif
  end
endmodule
