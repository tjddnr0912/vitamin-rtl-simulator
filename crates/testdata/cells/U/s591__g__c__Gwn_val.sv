module top;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam [64:0] j = 65'h1_0000_0000_0000_0009;
    for (genvar j = 0; j < 2; j++) begin : h
      localparam int K = j;
      initial #1 $display("val %m j=%0d K=%0d", j, K);
    end
    initial #5 $display("post %m %0d", j);
  end
  initial #100 $finish;
endmodule
