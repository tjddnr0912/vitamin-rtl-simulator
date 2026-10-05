module top;
  logic a = 0, b = 1; wire [1:0] y = {a, b}; wire c = 1'b1; wire [1:0] n = ~{a, b};
  initial begin
    $display("i0 y=%b c=%b n=%b", y, c, n);
    #1 $display("t=%0t y=%b c=%b n=%b", $time, y, c, n);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
