module top;
  logic a, b;
  always begin : chk
    logic x, z; x = a; z = b;
    unique if (x) begin end else if (z) begin end
    @(a or b);
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
