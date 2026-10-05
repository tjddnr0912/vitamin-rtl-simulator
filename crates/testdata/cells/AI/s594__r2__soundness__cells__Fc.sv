module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  localparam int L1 = fl(0);
  localparam U = X + {N{1'b0}};
  initial $display("L1=%0d U=%0d B=%0d", L1, U, $bits(U));
  initial #100 $finish;
endmodule
