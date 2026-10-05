module top;
  localparam logic [1:0][39:0] PP = {40'h10_0000_000C, 40'h1};
  if (PP[1] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
