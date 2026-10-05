module top;
  logic a = 0, b;
  logic [1:0] y;
  always_comb b = a;
  always_comb begin
    $display("evalcase t=%0t ab=%b%b", $time, a, b);
    unique case ({a, b})
      2'b00: y = 0;
      2'b11: y = 3;
    endcase
  end
  initial begin
    #5 a = 1;
    #1 $display("t=%0t y=%0d done", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
