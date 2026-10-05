module t;
  localparam int N = 9;
  localparam logic [7:0] L = '0 | (((8'd200 + 8'd100) >> 1) + ({N{8'hFF}} != 0));
  initial $display("L=%0d", L);
  initial #100 $finish;
endmodule
