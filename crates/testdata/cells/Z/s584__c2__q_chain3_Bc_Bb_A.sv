module top;
  logic a = 1'b0, b, c;
  logic [1:0] y;
  always_comb begin $display("Bc t=%0t", $time); c = b; end
  always_comb begin $display("Bb t=%0t", $time); b = a; end
  always_comb begin
    $display("A t=%0t ac=%b%b", $time, a, c);
    unique case ({a, c}) 2'b00: y = 0; 2'b11: y = 3; endcase
  end
  initial begin
    #5 a = 1'b1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
