// P4b: F1 boundary (LHS wider than 64 bits) + S2 parameter-array element holding x
module t;
  localparam [99:0] W = {36'h1, 64'h1};                    // bits 64 and 0 set
  parameter logic [3:0] PA [0:1] = '{4'b1x00, 4'b0000};
`ifndef NO_INSIDE
  localparam LW = W inside {4'b000?};                      // IEEE 0 (bit 64 must be 0)
`endif
  localparam LWQ = W ==? 4'b000?;                          // 0
  logic [3:0] v;
  initial begin
    v = 4'b1100;
`ifndef NO_INSIDE
    $display("LW=%b PA0in=%b", LW, v inside {PA[0]});
`endif
    $display("LWQ=%b PA0=%b PA0wq=%b PA0eq=%b", LWQ, PA[0], v ==? PA[0], v == PA[0]);
    #1 $finish;
  end
endmodule
