module top;
  reg [7:0] a;
  initial begin
    a = 4'b10xg;
    a = 4'b10g;
    a = 4'b1x4'b1;
    a = 8'hx F;
    a = 4'b1x 0;
    a = 'b ;
    a = 4 'b;
    $finish;
  end
endmodule
