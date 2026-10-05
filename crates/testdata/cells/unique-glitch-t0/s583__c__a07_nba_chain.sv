module top;
  logic clk = 0;
  logic a = 0, b = 0;
  logic [1:0] y;
  always @(posedge clk) a <= ~a;
  always @(a) b <= a;
  always_comb begin
    $display("eval t=%0t ab=%b%b", $time, a, b);
    unique case ({a, b})
      2'b00: y = 0;
      2'b11: y = 3;
    endcase
  end
  initial begin
    #5 clk = 1;
    #1 $display("t=%0t y=%0d done", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
