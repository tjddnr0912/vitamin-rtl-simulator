module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  initial begin
    #0 $display("i1 y=%b w=%b", y, w);
    a = 0; b = 1;
    #0 $display("i2 y=%b w=%b", y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
