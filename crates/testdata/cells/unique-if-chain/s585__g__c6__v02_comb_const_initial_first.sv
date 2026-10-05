module top;
  logic [1:0] y;
  initial begin
    $display("init t=%0t y=%b", $time, y);
    #0 $display("init#0 t=%0t y=%b", $time, y);
    #0 $display("init#0#0 t=%0t y=%b", $time, y);
  end
  always_comb y = 2'd3;
  initial #1 $finish;
endmodule
