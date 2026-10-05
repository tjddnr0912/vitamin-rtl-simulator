module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  for (genvar i = 0; i < (X + {N{1'b0}}) / 63; i++) begin : G
    initial $display("R: i=%0d", i);
  end
  initial #10 $finish;
endmodule
