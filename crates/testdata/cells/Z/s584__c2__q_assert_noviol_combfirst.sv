module top;
  logic [1:0] s, y;
  always_comb begin
    y = s;
    assert (s == 2'd1 || s == 2'd2);
  end
  initial begin
    s = 2'd1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
