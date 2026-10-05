// P5: F1 boundary (LHS wider than 64 bits; F1 under `!`) + S4 fill element against a 36-bit LHS
module t;
  localparam [99:0] W = {36'h1, 64'h1};                    // bits 64 and 0 set
`ifndef NO_INSIDE
  localparam LW = W inside {4'b000?};                      // IEEE 0
  localparam LN = !((4'd15 + 4'd1) inside {5'b1?000});     // IEEE 0
`endif
  localparam LWQ = W ==? 4'b000?;                          // 0
  logic [35:0] w36;
  initial begin
    w36 = 36'h8_0000_0001;
`ifndef NO_INSIDE
    $display("LW=%b LN=%b F1=%b F2=%b F3=%b", LW, LN, w36 inside {'x}, w36 inside {'z}, w36 inside {36'h0, 'x});
`endif
    $display("LWQ=%b FQ=%b", LWQ, w36 ==? 'x);
    #1 $finish;
  end
endmodule
