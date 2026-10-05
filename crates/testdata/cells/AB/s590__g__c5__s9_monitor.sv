module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial $monitor("M t=%0t y=%b", $time, y);
  initial begin
    $strobe("S t=%0t y=%b", $time, y);
    a = 0; b = 1;
    #1 b = 0;
    #1 $finish;
  end
  initial #100 $finish;
endmodule
