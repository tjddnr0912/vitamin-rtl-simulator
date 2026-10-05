module top;
  logic a, b;
  always begin : chk
    logic x, z; x = a; z = b;
    unique case ({x, z}) 2'b10: ; 2'b01: ; endcase
    @(a or b);
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
