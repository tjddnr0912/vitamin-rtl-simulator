module t;
  localparam N = 4;
  function automatic int g(input int a);
    logic signed [7:0] v;
    v = -4;
    g = (((a != 0) ? v : 8'sd0) + {N{1'b0}}) == 8'hFC;
  endfunction
  localparam L = g(1);
  initial $display("R: L=%0d", L);
  initial #10 $finish;
endmodule
