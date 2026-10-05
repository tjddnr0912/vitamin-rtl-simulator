module top;
  logic a; logic [31:0] y;
  function integer f(input logic x);
    integer v, n;
    n = $sscanf("42", "%d", v);
    return v + n;
  endfunction
  assign y = f(a);
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
