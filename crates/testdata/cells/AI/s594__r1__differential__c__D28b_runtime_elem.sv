module t;
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  localparam N = 4;
  logic [31:0] r32;
  initial begin
    r32 = AS[0] + {N{1'b0}};
    $display("R: r32=%0d d=%0d", r32, AS[0] + {N{1'b0}});
  end
  initial #10 $finish;
endmodule
