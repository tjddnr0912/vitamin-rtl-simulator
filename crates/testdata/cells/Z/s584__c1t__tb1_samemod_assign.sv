module top;
  logic [1:0] s, y;
  wire [1:0] w;
  assign w = s;
  initial begin
    $display("I0 t=%0t s=%b w=%b", $time, s, w);
    s = 2'd1;
    $display("I1 t=%0t s=%b w=%b", $time, s, w);
    #0 $display("I2 t=%0t s=%b w=%b", $time, s, w);
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  always_comb begin
    $display("C t=%0t w=%b", $time, w);
    y = 0;
    unique case (w) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #100 $finish;
endmodule
