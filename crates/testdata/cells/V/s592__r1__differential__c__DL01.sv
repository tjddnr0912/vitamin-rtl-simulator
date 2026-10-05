module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      wire [7:0] w;
    end else begin : b
      wire [3:0] w;
    end
    localparam integer K = 2;
  end
  initial #1 $display("@bits=%0d", $bits(g.b.w));
  initial #10 $finish;
endmodule
