module top;
  logic [1:0] s [2], y [2];
  for (genvar i = 0; i < 2; i++) begin : g
    always_comb begin
      y[i] = 0;
      unique case (s[i]) 2'd1: y[i] = 1; 2'd2: y[i] = 2; endcase
    end
  end
  initial begin s[0] = 2'd1; s[1] = 2'd2; #2 s[0] = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
