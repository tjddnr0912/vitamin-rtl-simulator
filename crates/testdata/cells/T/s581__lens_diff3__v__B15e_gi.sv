module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  if (longint'(P40) ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
