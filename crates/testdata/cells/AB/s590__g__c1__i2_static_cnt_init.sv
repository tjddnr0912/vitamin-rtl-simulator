module top;
  logic a; logic [7:0] y;
  function logic [7:0] cnt(input logic x);
    static logic [7:0] n = 0;
    n = n + 1;
    return n;
  endfunction
  assign y = cnt(a);
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    a = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
