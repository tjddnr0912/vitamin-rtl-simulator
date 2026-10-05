module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  if (1) begin
    for (genvar i = 0; i < 2; i++) begin : g
      initial #3 $display("n3 body %m i=%0d", i);
    end
    initial #4 $display("n3 if-post %m i=%0d", i);
  end
  initial #5 $display("n3 post i=%0d b=%0d", i, $bits(i));
  initial #100 $finish;
endmodule
