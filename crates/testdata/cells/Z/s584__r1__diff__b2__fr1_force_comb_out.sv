module top;
  logic [1:0] a = 2'd1;
  logic [1:0] y;
  wire [1:0] v = y;
  always_comb y = a;
  initial begin
    force y = 2'd3;
    #1 $display("f t=%0t y=%b v=%b", $time, y, v);
    release y;
    #1 $display("r t=%0t y=%b v=%b", $time, y, v);
    a = 2'd2;
    #1 $display("a t=%0t y=%b v=%b", $time, y, v);
    $finish;
  end
endmodule
