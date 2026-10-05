module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  if (P40[36] ==? 2'b?1) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
