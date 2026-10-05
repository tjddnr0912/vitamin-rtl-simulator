`timescale 1ns/1ns
module late;
  logic [35:0] v36 = 36'hF_0000_0000;
  logic [7:0] v8 = 8'hFF;
  function automatic logic [35:0] f36(); return 36'hF_0000_0000; endfunction
  for (genvar i = 0; i < 1; i++) begin : g
    logic [35:0] gv = 36'hF_0000_0000;
  end
endmodule
module t;
  late uL();
  wire a1 = t.uL.v36 ==? 'x;
`ifndef NO_INSIDE
  wire a2 = t.uL.v36 inside {'x};
`endif
  initial begin
    #1;
    $display("A %b", a1);
    $display("D %b %b", uL.v36 ==? 'x, uL.v8 ==? '1);
    $display("H %b %b %b %b", t.uL.f36() ==? 'x, t.uL.g[0].gv ==? 'x, uL.g[0].gv ==? 'x, t.uL.v36 ==? 36'hx);
`ifndef NO_INSIDE
    $display("AI %b DI %b HI %b", a2, uL.v36 inside {'x}, uL.g[0].gv inside {'z});
`endif
`ifdef U
    $display("U %b", t.uL.v36 ==? 'hx);
`endif
    #1 $finish;
  end
endmodule
