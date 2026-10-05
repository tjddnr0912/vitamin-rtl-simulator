module t;
  logic a, b; int y;
  initial begin
    a = 1; b = 1;
    #1;
    unique if (a) y = 1;
    else if (b) y = 2;
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
