// P2b: `inside` range bound alone (PRE value) + size-cast path (S6)
module t;
  logic [((4'd15 + 4'd1) inside {5'b1?000}) : 0] rb;   // [1:0] -> 2
  logic [3:0] a, b;
  initial begin
    a = 4'd1; b = 4'b1010;
    $display("bits(rb)=%0d", $bits(rb));
    $display("SC3=%b", 8'(a + (b inside {4'b1x0x})));  // 00000010
    #2 $finish;
  end
endmodule
