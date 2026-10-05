module t;
  localparam N = 4;
  function automatic int f(input int a);
    localparam N = 16;
    f = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);
  endfunction
  localparam L = f(0);
  initial $display("R: L=%0d", L);
  initial #10 $finish;
endmodule
