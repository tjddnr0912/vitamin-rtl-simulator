module top;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  if (ES ==? 8'b1111_1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
