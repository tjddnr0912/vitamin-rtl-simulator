module top;
  localparam real i = 4.0;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #1 $display("cmp %m eq1=%0d lt=%0d", (i == 1), (i < 1));
  end
  initial #5 $display("post %f", i);
  initial #100 $finish;
endmodule
