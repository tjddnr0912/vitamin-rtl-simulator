module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  begin : b
    localparam [64:0] i = 65'h2_0000_0000_0000_0003;
    for (genvar i = 0; i < 2; i++) begin : g
      initial #3 $display("n14 body %m i=%0d", i);
    end
    initial #4 $display("n14 b-post %m i=%0d", i);
  end
  initial #5 $display("n14 post i=%0d", i);
  initial #100 $finish;
endmodule
