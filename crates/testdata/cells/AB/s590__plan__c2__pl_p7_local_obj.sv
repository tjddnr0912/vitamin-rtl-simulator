class C;
  function logic [1:0] g(input logic x, input logic z);
    $display("g t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
endclass
module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    C c;
    c = new;
    return c.g(x, z);
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
