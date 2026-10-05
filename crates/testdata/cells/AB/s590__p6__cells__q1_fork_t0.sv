module top;
  logic [1:0] a;
  wire [1:0] y;
  function logic [1:0] f(input logic [1:0] x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign y = f(a);
  initial begin
    fork
      a = 2'b01;
      #1 a = 2'b10;
    join_none
    $display("i0 y=%b", y);
    #2 $display("i2 y=%b", y);
    $finish;
  end
  initial #100 $finish;
endmodule
