module top;
  logic a;
  wire w;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  assign w = f(a);
  initial begin force w = 1'b1; a = 1'b0; #1 $display("t1 w=%b", w); release w; #1 $display("t2 w=%b", w); end
  initial #10 $finish;
endmodule
