module top; logic [0:0] a; logic y; function logic f(input logic x); $display("f t=%0t x=%b", $time, x); return ~x; endfunction
  assign y = f(a);
  initial begin a = 0; #1 $display("t=%0t y=%b", $time, y); end
endmodule
