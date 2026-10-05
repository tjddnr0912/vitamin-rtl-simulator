// P8: const-domain x/z element with a DEFINITE known-bit mismatch that the slice's fold_region decline may turn loud
module t;
  localparam [99:0] W = {36'h1, 64'h1};
`ifndef NO_INSIDE
  localparam L1 = 4'b0100 inside {4'sb1?00};           // bit 3 mismatch -> 0
  localparam L3 = 4'b0100 inside {'b1?00};             // 0
  localparam LW2 = W inside {4'b000?};                 // bit 64 mismatch -> 0
  if (4'b0100 inside {4'sb1?00}) begin : g1y
    initial #1 $display("G1 then");
  end else begin : g1n
    initial #1 $display("G1 else");
  end
`endif
  initial begin
`ifndef NO_INSIDE
    $display("L1=%b L3=%b LW2=%b", L1, L3, LW2);
`endif
    #2 $finish;
  end
endmodule
