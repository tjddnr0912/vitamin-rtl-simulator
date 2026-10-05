module top;
  import "DPI-C" function int c_f(input int x);
  logic a; int y;
  assign y = c_f(int'(a));
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
