module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam [64:0] j = 65'h2_0000_0000_0000_0003;
    for (genvar j = 0; j < 1; j++) begin : h
      initial #2 $display("n15 inner %m i=%0d j=%0d", i, j);
    end
    initial #3 $display("n15 outer %m j=%0d", j);
  end
  initial #5 $display("n15 post i=%0d", i);
  initial #100 $finish;
endmodule
