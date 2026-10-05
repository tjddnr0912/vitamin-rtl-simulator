module top;
  wire w;
  logic [1:0] s;
  logic [1:0] y;
  assign w = 1'b1;
  always @(w) s = {1'b0, w};
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial begin #1 $display("t=1 y=%0d", y); #1 $finish; end
endmodule
