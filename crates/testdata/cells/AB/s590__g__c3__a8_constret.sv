module top;
  logic a; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return 2'b01;
  endfunction
  assign y = f(a);
  assign w = f(a);
  initial begin
    $display("i0 y=%b w=%b", y, w);
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
