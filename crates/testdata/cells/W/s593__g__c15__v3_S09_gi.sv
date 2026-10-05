module top;
  typedef logic signed [39:0] t40s;
  if (t40s'(40'hFF_FFFF_FFFC) ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
