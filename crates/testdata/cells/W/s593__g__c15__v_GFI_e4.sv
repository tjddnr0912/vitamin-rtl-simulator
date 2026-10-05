module top;

  for (genvar i = (4'd12 inside {4'b1?00}); i < 2; i++) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
