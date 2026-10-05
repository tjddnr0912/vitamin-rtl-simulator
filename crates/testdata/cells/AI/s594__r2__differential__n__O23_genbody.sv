module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  if (fz(1) == 1) begin : G
    localparam int L = ((X + {N{1'b0}}) == 8'hFC);
    initial $display("R: GL=%0d", L);
  end
  for (genvar i = 0; i < fz(2); i++) begin : F
    localparam int L = ((X + {N{1'b0}}) == 8'hFC);
    initial $display("R: F%0d=%0d", i, L);
  end
  initial #40 $finish;
endmodule
