module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f t=%0t x=%0d", $time, x);
    f = x + 4'd1;
  endfunction
  wire [3:0] h = f(a);
  wire [3:0] r = h ^ 4'd8;
  always @(r) $display("ev r=%0d t=%0t", r, $time);
  initial begin
    force h = 4'd5;
    a = 1;
    #3 release h;
    #1 $display("t4 h=%0d r=%0d", h, r);
    #1 a = 2;
    #1 $display("t6 h=%0d r=%0d", h, r);
    $finish;
  end
  initial #50 $finish;
endmodule
