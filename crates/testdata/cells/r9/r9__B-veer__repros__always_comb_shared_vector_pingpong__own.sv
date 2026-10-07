module top;
  logic [1:0]      any;
  logic [1:0][1:0] m;
  for (genvar k = 0; k < 2; k++) begin : BANKS
    logic a;                     // per-iteration temporary, then one write of the shared vector
    always_comb begin
      a = '0;
      for (int l = 0; l < 2; l++) a |= m[k][l];
      any[k] = a;
    end
  end
  initial begin #1 m = 4'b0110; #1 $display("any=%b", any); $finish; end
endmodule
