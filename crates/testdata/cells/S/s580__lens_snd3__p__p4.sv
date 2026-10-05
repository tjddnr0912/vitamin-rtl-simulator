module sub;
  logic [7:0] v8 = 8'b0110_0001;
  real r = 12.0;
  for (genvar i = 0; i < 1; i++) begin : g
    logic [7:0] gv = 8'b0110_0001;
  end
endmodule
module t;
  sub uL();
  string s;
  for (genvar i = 0; i < 1; i++) begin : g
    logic [7:0] gv = 8'b0110_0001;
  end
  initial #100 $finish;
  initial begin
    s = "a";
    #1;
`ifdef K1
    $display("K1 %b", t.uL.v8 ==? 8'b0110_000?);
`endif
`ifdef K2
    $display("K2 %b", s ==? 8'b0110_000?);
`endif
`ifdef K3
    $display("K3 %b", $sformatf("%s", s) ==? 8'b0110_000?);
`endif
`ifdef K4
    $display("K4 %b", g[0].gv ==? 8'b0110_000?);
`endif
`ifdef K5
    $display("K5 %b", t.uL.r ==? 4'b1?00);
`endif
`ifdef K6
    $display("K6 %b", t.uL.v8 inside {8'b0110_000?});
`endif
`ifdef K7
    $display("K7 %b", t.uL.r inside {4'b1?00});
`endif
`ifdef K8
    $display("K8 %b", uL.g[0].gv ==? 8'b0110_000?);
`endif
`ifdef K9
    $display("K9 %b", t.uL.v8 ==? 'x);
`endif
  end
endmodule
