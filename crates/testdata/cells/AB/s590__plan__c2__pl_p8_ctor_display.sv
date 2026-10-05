class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    $display("new t=%0t x=%b z=%b", $time, x, z);
    r = {x, z};
  endfunction
endclass
module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    D d;
    d = new(x, z);
    return d.r;
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
