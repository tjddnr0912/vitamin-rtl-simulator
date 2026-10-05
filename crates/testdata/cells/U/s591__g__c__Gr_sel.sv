module top;
  localparam real i = 2.5;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #1 $display("sel %m s0=%b s1=%b", i[0], i[1]);
  end
  initial #5 $display("post %f", i);
  initial #100 $finish;
endmodule
