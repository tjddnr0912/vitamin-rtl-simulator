module top;
  reg clk = 0;
  always #1 clk = ~clk;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g ap: assert property (@(posedge clk) 1'b0) else $display("@fail %m"); end
      default: begin : g  end
    endcase
    localparam K = 8;
  end

  initial #5 $finish;
endmodule
