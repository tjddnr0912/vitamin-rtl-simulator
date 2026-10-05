module top;
  logic a, b; logic [1:0] y1, y2;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y1 = f(a, b);
  assign y2 = f(b, a);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y1=%b y2=%b", $time, y1, y2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
