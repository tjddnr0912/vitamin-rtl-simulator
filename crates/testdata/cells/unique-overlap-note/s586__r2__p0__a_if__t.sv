module t;
  logic a, b; int y;
  initial begin
    a = 0; b = 0; y = 9;
    #1;
    priority0 if (a) y = 1;
    else if (b) y = 2;
    $display("y=%0d", y);
    #1 $finish;
  end
endmodule
