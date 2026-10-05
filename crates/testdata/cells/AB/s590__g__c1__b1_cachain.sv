module top;
  logic a, b; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  function logic [1:0] g(input logic [1:0] v);
    $display("g t=%0t v=%b", $time, v);
    return ~v;
  endfunction
  assign z = g(y);
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b z=%b", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
