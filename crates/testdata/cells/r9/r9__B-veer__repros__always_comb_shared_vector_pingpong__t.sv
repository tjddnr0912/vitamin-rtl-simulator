module top;
  logic [1:0]      any;        // each generate iteration's always_comb writes its own bit
  logic [1:0][1:0] m;          // never assigned before #1: x
  for (genvar k = 0; k < 2; k++) begin : BANKS
    always_comb begin
      any[k] = '0;
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];
    end
  end
  initial begin
    #1 m = 4'b0110;
    #1 $display("any=%b", any);
    $finish;
  end
endmodule
