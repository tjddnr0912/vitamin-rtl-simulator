module top;
  logic a, e, y; int n;
  task automatic tk; @(posedge e); endtask
  always_latch begin n = n + 1; if (a) y = a; $display("L t=%0t a=%b n=%0d", $time, a, n); tk(); $display("D t=%0t n=%0d", $time, n); end
  initial begin a = 0; e = 0; #5 a = 1; #2 e = 1; #5 $finish; end
endmodule
