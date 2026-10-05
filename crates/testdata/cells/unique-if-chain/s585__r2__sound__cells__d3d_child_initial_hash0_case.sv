module chk(input logic a, input logic b);
  logic [1:0] r;
  initial #0 begin r = 0; unique case (1'b1) a: r = 1; b: r = 2; endcase $display("chk t=%0t r=%0d", $time, r); end
endmodule
module top;
  logic a, b;
  chk u(.a(a), .b(b));
  initial begin a = 0; b = 1; #1 $finish; end
endmodule
