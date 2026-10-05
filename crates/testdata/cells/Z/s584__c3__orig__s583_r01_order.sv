module top;
  logic [1:0] r = 2'b01;
  logic [1:0] y;
  logic q = 0;
  always_comb begin
    $display("comb-eval t=%0t r=%b", $time, r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
  initial begin
    #5 r = 2'b00;
    $display("A-after-write t=%0t", $time);
    #0 $display("A-after-#0 t=%0t", $time);
  end
  initial begin
    #5 q <= 1;
    $strobe("strobe t=%0t r=%b", $time, r);
  end
  always @(q) $display("after-NBA q=%b t=%0t", q, $time);
  initial $monitor("monitor t=%0t r=%b q=%b", $time, r, q);
  initial #7 $finish;
  initial #100 $finish;
endmodule
