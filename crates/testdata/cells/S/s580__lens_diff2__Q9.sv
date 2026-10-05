`ifdef IV
 `define IN(a,b) ((a) ==? b)
`else
 `define IN(a,b) ((a) inside {b})
`endif
module t;
  localparam UE = 4'd15 + 4'd1;
  localparam N = -6;
  localparam NP1 = `IN(N, 4'sb1x1x);
  localparam NP2 = `IN(N, 4'b1x1x);
  localparam NP3 = `IN(N, 36'sh?_FFFF_FFFA);
  int n = -6;
  initial begin
    #1;
    $display("UEV %0d", UE); $display("UEB %0d", $bits(UE));
    $display("NP1 %b", NP1); $display("RN1 %b", `IN(n, 4'sb1x1x));
    $display("NP2 %b", NP2); $display("RN2 %b", `IN(n, 4'b1x1x));
    $display("NP3 %b", NP3); $display("RN3 %b", `IN(n, 36'sh?_FFFF_FFFA));
    #1 $finish;
  end
endmodule
