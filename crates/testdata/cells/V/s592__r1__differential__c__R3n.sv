module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      localparam integer Q = 20;
    end else begin : b
      localparam integer Q = 10;
    end
    localparam integer K = 2;
  end
  initial #1 $display("@Qa=%0d", g.a.Q);
  initial #10 $finish;
endmodule
