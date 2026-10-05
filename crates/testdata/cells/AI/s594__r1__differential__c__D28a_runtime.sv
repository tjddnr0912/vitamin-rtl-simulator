module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  logic [31:0] r32;
  initial begin
    r32 = X + {N{1'b0}};
    $display("R: r32=%0d d=%0d", r32, X + {N{1'b0}});
  end
  initial #10 $finish;
endmodule
