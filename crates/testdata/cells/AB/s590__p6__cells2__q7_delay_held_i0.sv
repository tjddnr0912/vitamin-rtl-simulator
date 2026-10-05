module top;
  logic a, b; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign #1 w = f(a, b);
  initial begin
    $display("i0 w=%b", w);
    a = 0; b = 1;
    #0 $display("i1 w=%b", w);
    #2 $display("t=%0t w=%b", $time, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
