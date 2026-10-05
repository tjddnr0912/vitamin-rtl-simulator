module top;
  typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;
  if (E0 ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
