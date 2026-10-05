module top;
  reg clk = 0;
  always #1 clk = ~clk;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      assert property (@(posedge clk) 1'b0) else $display("@a_fail %0t", $time);
    end else begin : b
      assert property (@(posedge clk) 1'b0) else $display("@b_fail %0t", $time);
    end
    localparam integer K = 2;
  end
  initial #4 $finish;
endmodule
