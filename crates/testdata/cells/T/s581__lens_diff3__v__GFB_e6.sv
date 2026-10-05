module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  for (genvar i = 0; i < 1 + (S64N ==? 4'sb1?00); i++) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
