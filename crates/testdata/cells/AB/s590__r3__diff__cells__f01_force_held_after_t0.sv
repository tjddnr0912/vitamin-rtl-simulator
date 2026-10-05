module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f t=%0t x=%0d", $time, x);
    f = x + 4'd1;
  endfunction
  wire [3:0] h = f(a);
  wire [3:0] r = h + 4'd1;
  always @(h) $display("ev h=%0d t=%0t", h, $time);
  always @(r) $display("ev r=%0d t=%0t", r, $time);
  initial begin
    a = 0;
    #2 a = 3;
    #2 force h = 4'd9;
    #2 a = 5;
    #2 release h;
    #2 $display("t10 h=%0d r=%0d", h, r);
    force r = 4'd0;
    #1 release r;
    #1 $display("t12 h=%0d r=%0d", h, r);
    $finish;
  end
  initial #50 $finish;
endmodule
