module top;
  logic [3:0] r = 4'd5;
  wire [3:0] k;
  assign k = 4'd6;
  function automatic logic [3:0] f(input logic [3:0] x);
    if (x === 4'bzzzz) $display("never");
    f = x + 1;
  endfunction
  wire [3:0] y1 = f(r);
  wire [3:0] y2 = f(k);
  initial begin @(y1) $display("ev y1=%0d t=%0t", y1, $time); end
  initial begin @(y2) $display("ev y2=%0d t=%0t", y2, $time); end
  always @(posedge y1[1]) $display("pos y1[1] t=%0t", $time);
  always @(posedge y2[0]) $display("pos y2[0] t=%0t", $time);
  initial if (y1 == 6) $display("init y1 ok"); else $display("init y1 bad %b", y1);
  initial #3 $finish;
endmodule
