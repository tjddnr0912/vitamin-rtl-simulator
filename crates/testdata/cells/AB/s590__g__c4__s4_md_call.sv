module top;
  logic a, b, en; wire y;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign y = en ? f(a) : 1'bz;
  assign y = en ? 1'bz : b;
  initial begin
    en = 1; a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    en = 0;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
