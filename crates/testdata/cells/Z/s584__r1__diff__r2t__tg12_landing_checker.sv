module top;
  logic [1:0] s = 2'd1;
  wire [1:0] w;
  logic [1:0] y;
  assign #0 w = s;
  always_comb begin
    $display("C t=%0t w=%b", $time, w);
    unique case (w)
      2'd0, 2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
