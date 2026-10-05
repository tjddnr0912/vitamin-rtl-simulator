module top;

  for (genvar i = 0; i < 1 + (4'd12 == 4'd12); i++) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
