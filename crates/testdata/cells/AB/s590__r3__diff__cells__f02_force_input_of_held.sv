module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f t=%0t x=%0d", $time, x);
    f = x + 4'd1;
  endfunction
  wire [3:0] w = a;
  wire [3:0] h = f(w);
  always @(h) $display("ev h=%0d t=%0t", h, $time);
  initial begin
    a = 2;
    #2 force w = 4'd7;
    #2 a = 4;
    #2 a = 6; release w;
    #2 $display("t8 w=%0d h=%0d", w, h);
    $finish;
  end
  initial #50 $finish;
endmodule
