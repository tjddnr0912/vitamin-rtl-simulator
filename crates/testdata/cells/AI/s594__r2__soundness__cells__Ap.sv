module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  localparam logic [7:0] A = fl(0);
  localparam int L = (A + (X + {N{1'b0}})) == 8'hFC;
  initial $display("A=%0d L=%0d", A, L);
  initial #100 $finish;
endmodule
