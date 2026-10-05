module top;
  logic a;
  logic [1:0] y;
  always @* begin
    $display("star t=%0t a=%b", $time, a);
    y = 0;
    unique case (a)
      1'b1: y = 1;
    endcase
  end
  initial begin
    #5 a = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
