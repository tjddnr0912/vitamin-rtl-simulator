module top;
  logic a; logic [7:0] y;
  function logic [7:0] f(input logic x);
    f = $random;
    if (x) f = 0;
  endfunction
  assign y = f(a);
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
