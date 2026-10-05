module top;
  logic a, b;
  initial begin : chk
    logic x, z; x = a; z = b;
    if (x) begin end else unique if (z) begin end
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
