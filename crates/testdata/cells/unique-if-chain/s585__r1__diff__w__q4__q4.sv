module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t after chain", $time);
    #1 unique if (a) r = 1;
    $display("t=%0t after lone", $time);
    #1 priority if (a) r = 1; else if (b) r = 2;
    $display("t=%0t after prio chain", $time);
    #1 $finish;
  end
endmodule
