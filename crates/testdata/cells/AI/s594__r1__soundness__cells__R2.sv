module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic fl1(input int a); logic r; fl1 = r; endfunction
  localparam int L = (((X + {N{1'b0}}) + {7'd0, fl1(2)}) == 8'hFC);
  localparam int K = (((X + {N{1'b0}}) + {{1{fl1(2)}}, 7'd0}) == 8'hFC);
  initial $display("L=%0d K=%0d", L, K);
  initial #100 $finish;
endmodule
