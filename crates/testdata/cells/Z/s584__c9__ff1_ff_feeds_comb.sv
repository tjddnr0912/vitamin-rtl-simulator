module top;
  logic clk, rst;
  logic [1:0] s;
  logic [1:0] y;
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  always_ff @(posedge clk or posedge rst) if (rst) s <= 2'd1; else s <= s;
  initial begin rst = 1'b0; clk = 1'b0; #0 rst = 1'b1; #1 $display("t=1 y=%0d", y); #1 $finish; end
endmodule
