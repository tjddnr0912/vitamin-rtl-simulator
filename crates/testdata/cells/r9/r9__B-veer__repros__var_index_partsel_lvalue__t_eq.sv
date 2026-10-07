module top;
  logic [1:0][7:0] b;
  logic [7:0] u [2];
  always_comb begin
    b = '0;
    for (int i = 0; i < 2; i++) b[i][7:0] = 8'h11 << i;   // variable element index, then a part-select
  end
  initial begin
    for (int i = 0; i < 2; i++) u[i][3:0] = 4'(i + 6);    // unpacked element, variable index, part-select
    for (int i = 0; i < 2; i++) u[i][7:4] = 4'(i + 1);
    #1 $display("b=%h u0=%h u1=%h", b, u[0], u[1]);
    $finish;
  end
endmodule
