module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  function automatic logic [7:0] f(input int a); f = a; endfunction
  localparam L1 = ((1'b1 ? f(252) : 8'd0) == (X + {N{1'b0}}));
  localparam L2 = ((1'b0 ? f(0) : (X + {N{1'b0}})) >> 2);
  initial $display("R: L1=%0d L2=%0d", L1, L2);
  initial #10 $finish;
endmodule
