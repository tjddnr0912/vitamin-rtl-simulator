module top;
  logic a, b; logic [1:0] y; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    if (x) return 2'd1; else return 2'd2;
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  initial begin
    $display("i0 y=%b w=%b", y, w);
    a = 1;
    $display("i1 y=%b w=%b", y, w);
    #0 $display("i2 y=%b w=%b", y, w);
    #1 $display("t=%0t y=%b w=%b", $time, y, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
