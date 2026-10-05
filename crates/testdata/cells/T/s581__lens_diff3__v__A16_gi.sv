module top;

  if ($clog2(40) ==? 'sb1?0) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
