module top;
  localparam integer N = 1;
  if (1) begin : g
    for (genvar i = 0; i < N; i++) begin : L
      wire [3:0] w;
    end
    localparam integer N = 3;
  end
  initial #1 $display("@bits=%0d", $bits(g.L[0].w));
  initial #10 $finish;
endmodule
