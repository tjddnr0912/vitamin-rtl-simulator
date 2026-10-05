`define CH(A,B) unique if (A) r = 1; else if (B) r = 2;
module top;
  logic a = 0, b = 0, c = 0, d = 0;
  int r, k;
  task automatic t();
    k = 0;
    do begin unique if (a) r = 1; else if (b) r = 2; k++; end while (k < 2);
  endtask
  initial begin
    #1 t(); $display("t=%0t do-while k=%0d", $time, k);
    #1 unique if (c) r = 1; else if (1'b1) begin unique if (a) r = 3; else if (b) r = 4; end
    $display("t=%0t nested-in-member", $time);
    #1 `CH(a, b) `CH(c, d)
    $display("t=%0t two-macro-one-line", $time);
    #1 priority if (a) r = 1; else unique if (b) r = 2; else if (c) r = 3; else unique0 if (d) r = 4;
    $display("t=%0t mixed-qualifiers", $time);
    #1 $finish;
  end
endmodule
