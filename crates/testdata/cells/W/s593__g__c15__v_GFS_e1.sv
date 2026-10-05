module top;

  for (genvar i = 0; i < 4; i = i + 1 + ((4'd15 + 4'd1) ==? 5'b1?000)) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
