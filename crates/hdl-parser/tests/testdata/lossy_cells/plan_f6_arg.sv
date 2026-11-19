module s(input logic [127:0] i); initial #2 $display("B i=%h", i); endmodule
module t; function automatic logic [127:0] f(input logic [127:0] a); return a; endfunction
  logic [127:0] v; s u(.i('x));
  initial begin v = f('x); #1 $display("A v=%h", v); end
  initial #100 $finish;
endmodule
