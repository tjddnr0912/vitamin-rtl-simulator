module top;
  logic [1:0] y;
  always_comb y = 2'd3;
  initial begin
    $display("i0 y=%b", y);
    #0 $display("i1 y=%b", y);
    #0 $display("i2 y=%b", y);
    #0 $display("i3 y=%b", y);
  end
  initial #1 $finish;
endmodule
