// P2a: `==?` operator in the const domain (attribution of F1's twin) + size-cast path (S6)
module t;
  localparam L2 = (4'd15 + 4'd1) ==? 5'b1?000;    // 1
  localparam L4 = (4'd0 - 4'd1) ==? 5'b1111?;      // 1
  logic [((4'd15 + 4'd1) ==? 5'b1?000) : 0] rbq;  // [1:0] -> 2
  if ((4'd15 + 4'd1) ==? 5'b1?000) begin : gq
    initial #1 $display("GQ then");
  end else begin : gn
    initial #1 $display("GQ else");
  end
  logic [3:0] a, b;
  initial begin
    a = 4'd1; b = 4'b1010;
    $display("L2=%b L4=%b bits(rbq)=%0d", L2, L4, $bits(rbq));
    $display("SC1=%b SC2=%b", 8'(a + (b ==? 4'b1x0x)), 8'(a + (b !=? 4'b1x0x)));
    #2 $finish;
  end
endmodule
