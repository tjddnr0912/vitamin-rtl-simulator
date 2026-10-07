module top;
  logic [1:0]      any;
  logic [1:0][1:0] m;
  always_comb begin              // one process writes both elements
    for (int k = 0; k < 2; k++) begin
      any[k] = '0;
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];
    end
  end
  initial begin #1 m = 4'b0110; #1 $display("any=%b", any); $finish; end
endmodule
