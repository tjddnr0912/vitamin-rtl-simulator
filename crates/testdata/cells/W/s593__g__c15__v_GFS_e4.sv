module top;

  for (genvar i = 0; i < 4; i = i + 1 + (4'd12 inside {4'b1?00})) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
