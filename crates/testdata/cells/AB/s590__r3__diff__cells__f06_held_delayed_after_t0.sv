module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f t=%0t x=%0d", $time, x);
    f = x + 4'd1;
  endfunction
  wire [3:0] y;
  assign #2 y = f(a);
  wire [3:0] z = y + 4'd1;
  always @(y) $display("ev y=%0d t=%0t", y, $time);
  initial begin a = 1; #3 a = 2; #1 a = 3; #4 $display("t8 y=%0d z=%0d", y, z); $finish; end
  initial #50 $finish;
endmodule
