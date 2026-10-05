module top;
  logic [1:0] src = 2'd1;
  if (1) begin : g0
    logic [1:0] v;
    always_comb v = src;
  end
  if (1) begin : g1
    logic [1:0] v;
    always_comb v = g0.v;
  end
  if (1) begin : g2
    logic [1:0] y;
    always_comb begin
      unique case (g1.v)
        2'd1: y = 2'd1;
        2'd2: y = 2'd2;
      endcase
    end
  end
  initial #1 $display("t=%0t y=%0d", $time, g2.y);
  initial #5 $finish;
endmodule
