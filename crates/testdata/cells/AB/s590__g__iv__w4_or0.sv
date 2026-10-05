module top; logic a; logic y; function logic f(input logic x); $display("f t=%0t x=%b", $time, x); return ~x; endfunction
  assign y = f(a | 1'b0);
  initial begin a = 0; #1 $display("t=%0t y=%b", $time, y); end
endmodule
