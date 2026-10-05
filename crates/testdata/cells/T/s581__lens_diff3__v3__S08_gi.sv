module top;
  typedef logic [63:0] t64u;
  if (t64u'(-64'sd4) ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
