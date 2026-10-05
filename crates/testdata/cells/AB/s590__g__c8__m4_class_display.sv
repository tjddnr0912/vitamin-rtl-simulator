class C;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
endclass
module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  C obj; initial obj = new;
  assign y = obj.f(a, b);
  assign w = obj.f(a, b);
  initial begin
    $display("i0 y=%b w=%b", y, w);
    a = 0; b = 1;
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
