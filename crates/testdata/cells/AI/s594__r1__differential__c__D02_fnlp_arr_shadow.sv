module t;
  localparam logic [3:0] A [2] = '{4'd1, 4'd2};
  function automatic int f(input int a);
    localparam logic [15:0] A [2] = '{16'd0, 16'd2};
    f = ((((A[0] + 8'd255 + 8'd1) >> 1)) == 128);
  endfunction
  localparam L = f(0);
  initial $display("R: L=%0d", L);
  initial #10 $finish;
endmodule
