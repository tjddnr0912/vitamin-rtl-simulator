module top;
  reg [7:0] a, b, c, d, e, f, g;
  initial begin
    a = 8'd1x;
    b = 8'dx1;
    c = 8'b_x1;
    d = 'sx;
    e = 4'b1x4;
    f = 8'o9x;
    g = 8'd?_?;
    $display("E %b %b %b %b %b %b %b", a, b, c, d, e, f, g);
    $finish;
  end
endmodule
