program p(input logic a, input logic b);
  logic [1:0] r;
  initial begin r = 0; unique case (1'b1) a: r = 1; b: r = 2; endcase $display("p t=%0t r=%0d", $time, r); end
  initial begin #2 r = 0; unique case (1'b1) a: r = 1; b: r = 2; endcase $display("p t=%0t r=%0d", $time, r); end
endprogram
module top;
  logic a, b;
  p u(.a(a), .b(b));
  initial begin a = 0; b = 1; #1 b = 0; #3 $finish; end
endmodule
