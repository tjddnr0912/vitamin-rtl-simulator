module top;
  logic [1:0] s, y;
  initial begin
    $display("x0 y=%b", y);
    #0 $display("x1 y=%b", y);
    #0 $display("x2 y=%b", y);
  end
  initial s = 2'd1;
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #1 $finish;
endmodule
