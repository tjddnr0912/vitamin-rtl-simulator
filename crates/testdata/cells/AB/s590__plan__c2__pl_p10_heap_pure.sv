class C;
  function logic [1:0] g(input logic x, input logic z);
    return {x, z};
  endfunction
endclass
module top;
  logic a, b; logic [1:0] y;
  C obj;
  initial obj = new;
  assign y = obj.g(a, b);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    b = 0;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
