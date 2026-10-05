module top;
  logic a, b;
  logic [1:0] y;
  initial begin
    a = 1'b0;
    #5 a = 1'b1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  always_comb begin
    $display("A t=%0t ab=%b%b", $time, a, b);
    unique case ({a, b}) 2'b00: y = 0; 2'b11: y = 3; endcase
  end
  always_comb begin $display("B t=%0t", $time); b = a; end
  initial #100 $finish;
endmodule
