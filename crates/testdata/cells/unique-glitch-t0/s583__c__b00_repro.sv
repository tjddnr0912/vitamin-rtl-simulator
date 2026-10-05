module top;
  logic a, b;
  logic [1:0] r;
  initial begin
    a = 0; b = 0; r = 0;
    #1;
    unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain r=%0d", $time, r);
    #1;
    unique if (a) r = 1;
    $display("t=%0t single r=%0d", $time, r);
    #1;
    priority if (a) r = 1;
    $display("t=%0t prio r=%0d", $time, r);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
