module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    if (x) return 2'd1; else return 2'd2;
  endfunction
  assign y = f(a, b);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    a = 1; b = 0;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
