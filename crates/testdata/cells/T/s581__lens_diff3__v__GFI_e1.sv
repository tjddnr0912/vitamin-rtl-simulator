module top;

  for (genvar i = ((4'd15 + 4'd1) ==? 5'b1?000); i < 2; i++) begin : g
    initial $display("G=%0d", i);
  end
  initial #100 $finish;
endmodule
