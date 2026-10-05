module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam B1 = $bits({N{1'b1}});
  logic [(X + {N{1'b0}}) - 250 : 0] v;
  initial $display("R: B1=%0d vb=%0d", B1, $bits(v));
  initial #10 $finish;
endmodule
