module top;
  logic a, b; wire [1:0] y = {a, b}; wire [1:0] n = ~{a, b};
  initial begin
    $display("i0 y=%b n=%b", y, n);
    a = 0; b = 1;
    $display("i1 y=%b n=%b", y, n);
    #0 $display("i2 y=%b n=%b", y, n);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
