module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign #1 y = f(a, b);
  assign #1 w = f(a, b);
  initial begin
    $display("i0 y=%b w=%b", y, w);
    a = 0; b = 1;
    #0 $display("i1 y=%b w=%b", y, w);
    #2 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
