module top;
  logic [3:0] r = 4'd5;
  wire [3:0] k;
  assign k = 4'd6;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%0d t=%0t", x, $time);
    f = x + 1;
  endfunction
  wire [3:0] y1 = f(r);
  wire [3:0] y2 = f(k);
  wire [3:0] v2 = y2 + 1;
  initial $display("init y1=%0d y2=%0d v2=%0d", y1, y2, v2);
  initial #1 $display("#1 y1=%0d y2=%0d v2=%0d", y1, y2, v2);
  initial #3 $finish;
endmodule
