module t;
  logic a, b;
  logic [1:0] r;
  always_comb begin
    unique if (a) r = 2'd1;
    else if (b) r = 2'd2;
  end
  initial begin
    #1 a = 1; b = 1;
    #1 $display("t=%0t r=%0d", $time, r);
    #1 a = 0; b = 0;
    #1 $display("t=%0t r=%0d", $time, r);
    #1 $finish;
  end
endmodule
