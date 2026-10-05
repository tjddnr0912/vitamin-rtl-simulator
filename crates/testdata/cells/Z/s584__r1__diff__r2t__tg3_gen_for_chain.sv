module top;
  logic [1:0] src = 2'd1;
  for (genvar i = 0; i < 3; i++) begin : g
    logic [1:0] v;
    if (i == 0) begin : s
      always_comb v = src;
    end else if (i == 1) begin : s
      always_comb v = g[0].v;
    end else begin : s
      always_comb begin
        unique case (g[1].v)
          2'd1: v = 2'd1;
          2'd2: v = 2'd2;
        endcase
      end
    end
  end
  initial #1 $display("t=%0t v2=%0d", $time, g[2].v);
  initial #5 $finish;
endmodule
