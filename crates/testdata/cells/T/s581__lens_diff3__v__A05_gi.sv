module top;

  if ($signed(4'b1100) ==? 8'sb1111_1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
