module top;
  logic a, b;
  logic [1:0] y;
  always_comb begin
    $display("eval t=%0t ab=%b%b", $time, a, b);
    if (a) y = 1; else unique if (b) y = 2;
  end
  initial begin
    a = 1; b = 0;
    #2 a = 0;
    #0 b = 1;
    #1 a = 0; b = 0;
    #1 $finish;
  end
  initial #100 $finish;
endmodule
