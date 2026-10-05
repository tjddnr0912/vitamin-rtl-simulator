`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module late; logic [7:0] u8 = 8'b0101_0100; endmodule
module t;
  logic [7:0] u8n;
  late uL();
  wire a1 = `IN(t.uL.u8, 4'b?100);
  wire a2 = `IN(t.u8n, 4'b?100);
  initial #1000 $finish;
  initial begin
    u8n = 8'b0000_0100; #1;
    $display("A1 %b", a1); $display("A2 %b", a2);
`ifndef NOP
    $display("P1 %b", `IN(t.uL.u8, 4'b?100));
    $display("P2 %b", `IN(t.u8n, 4'b?100));
`endif
    $display("P3 %b", `IN(uL.u8, 8'b0101_?100));
    $display("P4 %b", `IN(t.uL.u8, 8'b0101_0100));
  end
endmodule
