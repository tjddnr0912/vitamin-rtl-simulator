module top;
  logic [1:0] a;
  logic [1:0] s;
  logic [1:0] y;
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  always @* s = a;
  initial begin a = 2'd1; #1 $display("t=1 y=%0d", y); #1 a = 2'd0; #1 $finish; end
endmodule
