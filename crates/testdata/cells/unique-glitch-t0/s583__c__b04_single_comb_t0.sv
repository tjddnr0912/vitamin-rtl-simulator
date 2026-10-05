module top;
  logic a;
  logic [1:0] y;
  always_comb begin
    $display("eval t=%0t a=%b", $time, a);
    unique if (a) y = 1;
  end
  initial begin
    a = 1;
    #2 a = 0;
    #0 a = 1;
    #1 a = 0;
    #1 $finish;
  end
  initial #100 $finish;
endmodule
