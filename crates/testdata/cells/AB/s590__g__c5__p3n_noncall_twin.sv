module top;
  logic a, b; wire [1:0] y;
  assign y = (a === 1'b1) ? 2'b10 : 2'b01;
  always @(posedge y[0]) $display("P t=%0t y=%b", $time, y);
  always @(negedge y[0]) $display("N t=%0t y=%b", $time, y);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    a = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
