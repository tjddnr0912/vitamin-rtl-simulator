module top;
  logic a; integer n = 0;
  function int g(input logic x);
    if (x) $fatal(1, "g fatal t=%0t", $time);
    return 1;
  endfunction
  initial begin
    a = 1;
    $strobe("S t=%0t g=%0d", $time, g(a));
    #5 $display("after t=%0t", $time);
  end
endmodule
